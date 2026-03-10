use sea_orm::{ConnectionTrait, DatabaseConnection};
use tracing::warn;
use xxhash_rust::xxh3::xxh3_64;

use crate::error::DbInfraError;

pub fn pg_lock_id(key: &str) -> i64 {
    xxh3_64(key.as_bytes()) as i64
}

/// Guard struct that represents a held Postgres advisory lock.
pub struct Guard {
    admin_pool: Option<DatabaseConnection>,
    lock_key: i64,
    released: bool,
}

impl Guard {
    fn postgres(admin_pool: DatabaseConnection, lock_key: i64) -> Self {
        Self {
            admin_pool: Some(admin_pool),
            lock_key,
            released: false,
        }
    }

    pub async fn release(mut self) -> Result<(), DbInfraError> {
        if self.released {
            return Ok(());
        }

        let Some(admin_pool) = &self.admin_pool else {
            self.released = true;
            return Ok(());
        };

        use sea_orm::{DatabaseBackend, Statement};

        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_advisory_unlock($1) AS unlocked",
            vec![self.lock_key.into()],
        );

        let result = admin_pool.query_one(stmt).await;

        match result {
            Ok(Some(row)) => {
                let unlocked: bool =
                    row.try_get("", "unlocked")
                        .map_err(|e| DbInfraError::Config {
                            message: format!("failed to read unlock result: {e}"),
                        })?;

                if !unlocked {
                    warn!(
                        code = "PG_UNLOCK_FALSE",
                        lock_key = self.lock_key,
                        "Advisory lock unlock returned false"
                    );
                }
            }
            Ok(None) => {
                warn!(
                    lock_key = self.lock_key,
                    "No result from advisory lock unlock query"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    lock_key = self.lock_key,
                    "Failed to unlock advisory lock"
                );
            }
        }

        self.released = true;
        Ok(())
    }
}

/// PostgreSQL advisory lock using admin pool
pub struct PgAdvisoryLock {
    admin_pool: DatabaseConnection,
    lock_key: i64,
}

impl PgAdvisoryLock {
    pub fn new(admin_pool: DatabaseConnection, key: &str) -> Self {
        let lock_key = pg_lock_id(key);

        Self {
            admin_pool,
            lock_key,
        }
    }

    pub async fn try_acquire(&mut self) -> Result<Option<Guard>, DbInfraError> {
        use sea_orm::{DatabaseBackend, Statement};

        let lock_stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_try_advisory_lock($1) AS locked",
            vec![self.lock_key.into()],
        );

        let result =
            self.admin_pool
                .query_one(lock_stmt)
                .await
                .map_err(|e| DbInfraError::Config {
                    message: format!("failed to acquire advisory lock: {e}"),
                })?;

        let locked: bool = match result {
            Some(row) => row
                .try_get("", "locked")
                .map_err(|e| DbInfraError::Config {
                    message: format!("failed to read lock result: {e}"),
                })?,
            None => {
                return Err(DbInfraError::Config {
                    message: "pg_try_advisory_lock returned no row".to_string(),
                })
            }
        };

        if !locked {
            return Ok(None);
        }

        Ok(Some(Guard::postgres(
            self.admin_pool.clone(),
            self.lock_key,
        )))
    }
}
