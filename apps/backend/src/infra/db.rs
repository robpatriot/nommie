use std::fs::OpenOptions;
use std::path::Path;
use std::time::{Duration, Instant};

use fd_lock::RwLock;
use migration::{migrate, MigrationCommand};
use sea_orm::{
    ConnectOptions,
    ConnectionTrait,
    Database,
    DatabaseBackend,
    DatabaseConnection,
    Statement,
    TransactionTrait, // <-- bring TransactionTrait into scope
};
use xxhash_rust::xxh3::xxh3_64;

use crate::config::db::{db_url, sqlite_file_path, DbOwner, DbProfile};
use crate::error::AppError;

/// Build the app DB *and* guarantee schema is current.
/// - SQLite file: migrate pre-pool under OS file lock, then build pool.
/// - SQLite memory: 1-conn pool; migrate; return.
/// - Postgres: **migrate as Owner owner (advisory lock)**, then build runtime pool for `owner`.
pub async fn bootstrap_db(
    profile: DbProfile,
    owner: DbOwner,
) -> Result<DatabaseConnection, AppError> {
    match profile {
        DbProfile::InMemory => {
            let mut opt = ConnectOptions::new("sqlite::memory:");
            opt.min_connections(1)
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(2))
                .sqlx_logging(true);

            let pool = Database::connect(opt).await?;
            migrate(&pool, MigrationCommand::Up)
                .await
                .map_err(|e| AppError::config(format!("SQLite memory migrate: {e}")))?;
            Ok(pool)
        }

        DbProfile::SqliteFile { .. } => {
            // Pre-pool: single-process migration with OS lock.
            migrate_sqlite_file_pre_pool(&profile).await?;
            // Then build the runtime pool.
            build_pool(profile, owner).await
        }

        DbProfile::Prod | DbProfile::Test => {
            // 1) Ensure schema as **Owner** owner (has create/alter perms)
            {
                let admin_pool = build_pool(profile.clone(), DbOwner::Owner).await?;
                migrate_pg_with_lock(&admin_pool, &profile).await?;
                // admin_pool drops here
            }
            // 2) Build runtime pool for the requested `owner`
            build_pool(profile, owner).await
        }
    }
}

/// Build the runtime pool/connection.
async fn build_pool(profile: DbProfile, owner: DbOwner) -> Result<DatabaseConnection, AppError> {
    match profile {
        DbProfile::SqliteFile { .. } => {
            let path = sqlite_file_path(&profile)?;
            let url = format!("sqlite:{}?mode=rwc", path.display());

            // Allow CI/local tuning.
            let max = std::env::var("NOMMIE_SQLITE_POOL_MAX")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(16);

            let mut opt = ConnectOptions::new(&url);
            opt.min_connections(0)
                .max_connections(max)
                .acquire_timeout(Duration::from_secs(2)) // fail fast (avoid 30s stalls)
                .idle_timeout(Duration::from_secs(5))
                .sqlx_logging(true);

            let pool = Database::connect(opt)
                .await
                .map_err(|e| AppError::config(format!("SQLite connect {}: {e}", path.display())))?;

            // Per-conn pragmas (WAL already ensured by migrator).
            pool.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA foreign_keys = ON;",
            ))
            .await?;
            pool.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA busy_timeout = 500;",
            ))
            .await?;

            Ok(pool)
        }

        DbProfile::InMemory => unreachable!("handled in bootstrap_db"),

        DbProfile::Prod | DbProfile::Test => {
            let url = db_url(profile, owner)?;
            let max = std::env::var("NOMMIE_PG_POOL_MAX")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(20);

            let mut opt = ConnectOptions::new(&url);
            opt.min_connections(2)
                .max_connections(max)
                .acquire_timeout(Duration::from_secs(2))
                .idle_timeout(Duration::from_secs(30))
                .sqlx_logging(true);

            let pool = Database::connect(opt).await?;
            Ok(pool)
        }
    }
}

/// Dedicated SQLite connection for migration with appropriate PRAGMAs.
async fn open_sqlite_dedicated_for_migration(path: &Path) -> Result<DatabaseConnection, AppError> {
    let url = format!("sqlite:{}?mode=rwc", path.display());
    let mut opt = ConnectOptions::new(&url);
    opt.min_connections(1)
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(2))
        .sqlx_logging(true);

    let conn = Database::connect(opt)
        .await
        .map_err(|e| AppError::config(format!("connect({}): {e}", path.display())))?;

    conn.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA foreign_keys = ON;",
    ))
    .await?;
    conn.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA busy_timeout = 500;",
    ))
    .await?;
    conn.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA journal_mode = WAL;",
    ))
    .await?;
    conn.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA synchronous = NORMAL;",
    ))
    .await?;

    Ok(conn)
}

async fn sqlite_schema_exists(exec: &impl ConnectionTrait) -> bool {
    exec.query_one(Statement::from_string(
        DatabaseBackend::Sqlite,
        "SELECT 1 FROM sqlite_master \
         WHERE type='table' AND name='seaql_migrations' \
         LIMIT 1"
            .to_string(),
    ))
    .await
    .ok()
    .flatten()
    .is_some()
}

/// Pre-pool migration for SQLite file: OS **try-lock** with timeout + single `BEGIN IMMEDIATE`.
/// We run the migrator against the same connection that owns the outer txn, and explicitly COMMIT/ROLLBACK.
async fn migrate_sqlite_file_pre_pool(profile: &DbProfile) -> Result<(), AppError> {
    use std::io;

    let path = sqlite_file_path(profile)?;
    let lock_path = path.with_extension("migrate.lock");

    // Fast path (no lock).
    let temp = open_sqlite_dedicated_for_migration(&path).await?;
    if sqlite_schema_exists(&temp).await {
        return Ok(());
    }
    drop(temp);

    // Try-lock loop with backoff and hard timeout (~800ms worst).
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false) // lock file is a marker; don't clobber
        .open(&lock_path)
        .map_err(|e| AppError::config(format!("open lock {}: {e}", lock_path.display())))?;
    let mut lock = RwLock::new(file); // lock itself must be mutable for try_write()

    let start = Instant::now();
    let mut attempts = 0usize;
    let guard = loop {
        match lock.try_write() {
            Ok(g) => break g,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                attempts += 1;
                let delay_ms = (20u64)
                    .saturating_mul(1u64 << attempts.saturating_sub(1))
                    .min(200);
                if start.elapsed() + Duration::from_millis(delay_ms) > Duration::from_millis(800) {
                    return Err(AppError::config(format!(
                        "sqlite migration lock timeout after {:?} ({} attempts)",
                        start.elapsed(),
                        attempts
                    )));
                }
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                continue;
            }
            Err(e) => {
                return Err(AppError::config(format!(
                    "acquire sqlite migration lock {}: {e}",
                    lock_path.display()
                )))
            }
        }
    };
    // OS lock released when `guard` is dropped.

    // Second check under the lock.
    let conn = open_sqlite_dedicated_for_migration(&path).await?;
    if sqlite_schema_exists(&conn).await {
        drop(guard);
        return Ok(());
    }

    // BEGIN IMMEDIATE with bounded retries (<= ~650ms worst).
    let mut begin_attempts = 0usize;
    loop {
        match conn
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "BEGIN IMMEDIATE".to_string(),
            ))
            .await
        {
            Ok(_) => break,
            Err(e) if e.to_string().contains("database is locked") && begin_attempts < 6 => {
                begin_attempts += 1;
                let delay_ms = (20u64)
                    .saturating_mul(1u64 << (begin_attempts - 1))
                    .min(200);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                continue;
            }
            Err(e) => {
                drop(guard);
                return Err(AppError::config(format!("BEGIN IMMEDIATE failed: {e}")));
            }
        }
    }

    // Final check inside the outer write txn (same connection).
    if sqlite_schema_exists(&conn).await {
        let _ = conn
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ROLLBACK".to_string(),
            ))
            .await;
        drop(guard);
        return Ok(());
    }

    // Run migrator on the *same connection* (its inner txns are savepoints).
    let mig_res = migrate(&conn, MigrationCommand::Up).await;

    // COMMIT/ROLLBACK the outer txn started by BEGIN IMMEDIATE.
    match mig_res {
        Ok(()) => {
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "COMMIT".to_string(),
            ))
            .await
            .map_err(|e| AppError::config(format!("SQLite commit failed: {e}")))?;
            drop(guard);
            Ok(())
        }
        Err(e) => {
            let _ = conn
                .execute(Statement::from_string(
                    DatabaseBackend::Sqlite,
                    "ROLLBACK".to_string(),
                ))
                .await;
            drop(guard);
            Err(AppError::config(format!("SQLite migration failed: {e}")))
        }
    }
}

fn pg_lock_id(key: &str) -> i64 {
    xxh3_64(key.as_bytes()) as i64
}

/// Safe Postgres existence check for `seaql_migrations` without erroring if missing.
async fn pg_has_migration_table(conn: &impl ConnectionTrait) -> Result<bool, AppError> {
    // `to_regclass` returns NULL if the relation doesn't exist.
    let row = conn
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT to_regclass('public.seaql_migrations') IS NOT NULL AS exists".to_string(),
        ))
        .await
        .map_err(|e| AppError::config(format!("pg_has_migration_table: {e}")))?;
    let exists = row
        .and_then(|r| r.try_get::<bool>("", "exists").ok())
        .unwrap_or(false);
    Ok(exists)
}

/// Postgres migration under advisory lock pinned to one physical connection.
async fn migrate_pg_with_lock(
    pool: &DatabaseConnection,
    profile: &DbProfile,
) -> Result<(), AppError> {
    // Pin a single physical connection with a txn.
    let txn = pool
        .begin()
        .await
        .map_err(|e| AppError::config(format!("pin PG conn: {e}")))?;

    let key = format!("nommie:migrate:{:?}", profile);
    let id = pg_lock_id(&key);
    let deadline = Instant::now() + Duration::from_secs(3);

    // Short retries on pg_try_advisory_lock.
    loop {
        let row = txn
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                format!("SELECT pg_try_advisory_lock({}) AS ok", id),
            ))
            .await
            .map_err(|e| AppError::config(format!("pg_try_advisory_lock: {e}")))?;
        let ok = row
            .and_then(|r| r.try_get::<bool>("", "ok").ok())
            .unwrap_or(false);
        if ok {
            break;
        }
        if Instant::now() >= deadline {
            // Couldn't get the lock quickly—another worker may be migrating.
            txn.rollback().await.ok();
            // Check outside a txn: if migrations table exists, consider schema ready.
            if pg_has_migration_table(pool).await.unwrap_or(false) {
                return Ok(());
            }
            return Err(AppError::config(
                "pg advisory lock timeout and no schema present",
            ));
        }
        tokio::time::sleep(Duration::from_millis(60)).await;
    }

    // With the advisory lock held, just run the migrator unconditionally.
    // SeaORM migrations are idempotent: this is safe and avoids TOCTOU checks.
    migrate(&txn, MigrationCommand::Up)
        .await
        .map_err(|e| AppError::config(format!("PG migrate failed: {e}")))?;

    // Explicit unlock + commit.
    let _ = txn
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT pg_advisory_unlock({})", id),
        ))
        .await;
    txn.commit()
        .await
        .map_err(|e| AppError::config(format!("PG commit failed: {e}")))?;
    Ok(())
}
