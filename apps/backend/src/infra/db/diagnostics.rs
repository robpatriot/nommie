// No top-level imports needed - all imports are within the modules

/// SQLite diagnostics and connection tracking
pub mod sqlite_diagnostics {
    use std::process;
    use std::time::Instant;

    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    use tracing::info;

    use crate::error::AppError;

    /// Generate a short random hex ID for pool/connection tracking
    pub fn generate_short_id() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: u32 = rng.gen();
        format!("{:08x}", bytes)[..8].to_string()
    }

    /// Get current process ID
    pub fn current_pid() -> u32 {
        process::id()
    }

    /// Generate a consistent connection-based identifier for logging correlation
    /// Uses readily available connection properties: PID + backend + connection pointer
    pub fn connection_id<C: ConnectionTrait>(conn: &C) -> String {
        let pid = current_pid();
        let backend = match conn.get_database_backend() {
            DatabaseBackend::Sqlite => "sq",
            DatabaseBackend::Postgres => "pg",
            DatabaseBackend::MySql => "my",
        };
        let conn_ptr = conn as *const _ as usize;
        format!("{}-{}-{:x}", pid, backend, conn_ptr % 0xFFFF) // Keep last 4 hex digits
    }

    /// Log PRAGMA values for SQLite diagnostics
    pub async fn log_pragma_snapshot<C: ConnectionTrait>(
        conn: &C,
        pool_type: &str,
    ) -> Result<(), AppError> {
        let pid = current_pid();

        // Infer database type from connection
        let db_type = match conn.get_database_backend() {
            DatabaseBackend::Sqlite => "sqlite",
            DatabaseBackend::Postgres => "postgres",
            DatabaseBackend::MySql => {
                return Err(AppError::config_msg(
                    "MySQL not supported",
                    "MySQL database backend is not supported, only SQLite and PostgreSQL are supported",
                ));
            }
        };

        let pragmas = [
            "journal_mode",
            "synchronous",
            "busy_timeout",
            "locking_mode",
            "mmap_size",
        ];

        for pragma in &pragmas {
            let query = format!("PRAGMA {};", pragma);
            if let Ok(Some(row)) = conn
                .query_one(Statement::from_string(DatabaseBackend::Sqlite, query))
                .await
            {
                // Try different types that this PRAGMA might return
                let formatted_value = row
                    .try_get::<String>("", pragma)
                    .ok()
                    .or_else(|| row.try_get::<i32>("", pragma).ok().map(|v| v.to_string()))
                    .or_else(|| row.try_get::<i64>("", pragma).ok().map(|v| v.to_string()));

                if let Some(value) = formatted_value {
                    info!(
                        pid = pid,
                        db_type = db_type,
                        pool_type = pool_type,
                        pragma = pragma,
                        value = value,
                        "PRAGMA snapshot"
                    );
                }
            }
        }

        Ok(())
    }

    /// Probe SQLite lock acquisition
    pub async fn sqlite_lock_probe<C: ConnectionTrait>(
        conn: &C,
        pool_id: &str,
    ) -> Result<(), AppError> {
        let pid = current_pid();
        let start = Instant::now();

        let result = conn
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "BEGIN IMMEDIATE; ROLLBACK;".to_string(),
            ))
            .await;
        let elapsed_ms = start.elapsed().as_millis();

        match result {
            Ok(_) => {
                info!(
                    pid = pid,
                    pool_id = pool_id,
                    elapsed_ms = elapsed_ms,
                    acquired = true,
                    "SQLite lock probe success"
                );
                Ok(())
            }
            Err(e) => {
                info!(pid=pid, pool_id=pool_id, elapsed_ms=elapsed_ms, acquired=false, error=%e, "SQLite lock probe failed");
                Err(crate::infra::db::core::config_error_with_context(
                    "SQLite lock probe failed",
                    e,
                ))
            }
        }
    }

    /// Redact SQL statement for logging (replace values with ?)
    pub fn redact_sql_preview(sql: &str) -> String {
        // Basic redaction - replace quoted strings and numbers with ?
        let mut redacted = sql.to_string();

        // Replace single-quoted strings
        let mut chars: Vec<char> = redacted.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '\'' {
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                if i < chars.len() {
                    for item in chars.iter_mut().take(i).skip(start + 1) {
                        *item = '?';
                    }
                }
            }
            i += 1;
        }

        // Replace double-quoted strings
        let redacted_str: String = chars.into_iter().collect();
        redacted = redacted_str;

        let mut chars: Vec<char> = redacted.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '"' {
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    i += 1;
                }
                if i < chars.len() {
                    for item in chars.iter_mut().take(i).skip(start + 1) {
                        *item = '?';
                    }
                }
            }
            i += 1;
        }

        let result: String = chars.into_iter().collect();
        result.chars().take(80).collect()
    }
}

/// Migration counters - module-local atomics
pub mod migration_counters {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SCHEMA_CHECKS_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static MIGRATOR_RAN_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static BUSY_EVENTS_TOTAL: AtomicUsize = AtomicUsize::new(0);

    pub fn schema_check() {
        SCHEMA_CHECKS_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn migrator_ran() {
        MIGRATOR_RAN_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn busy_event() {
        BUSY_EVENTS_TOTAL.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(feature = "regression-tests")]
pub mod test_infra_counters {
    use std::sync::atomic::{AtomicUsize, Ordering};
    pub static DID_RUN_MIGRATE_HOOK: AtomicUsize = AtomicUsize::new(0);
    pub fn reset() {
        DID_RUN_MIGRATE_HOOK.store(0, Ordering::SeqCst);
    }
    pub fn bump() {
        DID_RUN_MIGRATE_HOOK.fetch_add(1, Ordering::SeqCst);
    }
}
