// Standard library imports
use std::future::Future;
use std::process;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use db_infra::error::DbInfraError;
// Local / external db-infra module imports
use db_infra::infra::db::diagnostics::{migration_counters, sqlite_diagnostics};
use db_infra::infra::db::locking::{BootstrapLock, InMemoryLock, PgAdvisoryLock, SqliteFileLock};
use db_infra::sanitize_db_url;
// External crate imports
use migration::MigrationCommand;
use rand::Rng;
use sea_orm::{
    ConnectOptions, Database, DatabaseConnection, SqlxPostgresConnector, SqlxSqliteConnector,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use tracing::{info, trace, warn};

// Use re-exported types from parent module
use super::{DbKind, DbOwner, RuntimeEnv};
// Internal crate imports
use crate::config::db::{make_conn_spec, validate_db_config, ConnectionSettings, DbSettings};
use crate::db::shared_pool_cache::get_or_create_shared_pool;
use crate::error::AppError;

/// Get database engine name for logging
fn get_db_engine(db_kind: DbKind) -> &'static str {
    match db_kind {
        DbKind::Postgres => "postgresql",
        DbKind::SqliteFile | DbKind::SqliteMemory => "sqlite",
    }
}

/// Get database path description for logging
fn get_db_path(db_kind: DbKind) -> String {
    match db_kind {
        DbKind::Postgres => "postgresql://...".to_string(),
        DbKind::SqliteFile => "sqlite file".to_string(),
        DbKind::SqliteMemory => "sqlite::memory:".to_string(),
    }
}

/// Retry a connection attempt with fixed interval delays
/// Returns the result of the last attempt after all retries are exhausted
async fn retry_connection<T, F, Fut>(
    mut connect_fn: F,
    max_attempts: u32,
    interval_ms: u64,
) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let mut last_error = None;

    for attempt in 1..=max_attempts {
        match connect_fn().await {
            Ok(result) => {
                if attempt > 1 {
                    info!(
                        "connection_retry=success attempts={} interval_ms={}",
                        attempt, interval_ms
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < max_attempts {
                    warn!(
                        "connection_retry=failed attempt={} max_attempts={} interval_ms={}",
                        attempt, max_attempts, interval_ms
                    );
                    tokio::time::sleep(Duration::from_millis(interval_ms)).await;
                }
            }
        }
    }

    let final_error = last_error.unwrap_or_else(|| {
        AppError::config_msg(
            "connection retry failed",
            "no error recorded after max attempts (this should not happen)",
        )
    });
    Err(final_error)
}

/// Build ordered session-level SQL statements for the given database kind and settings.
/// Note: SQLite file prerequisites (journal_mode, synchronous) are handled separately and not included here.
fn build_session_statements(db_kind: DbKind, settings: &DbSettings) -> Vec<String> {
    match (db_kind, settings) {
        (DbKind::SqliteFile | DbKind::SqliteMemory, DbSettings::Sqlite { busy_timeout_ms }) => {
            vec![
                "PRAGMA foreign_keys = ON;".to_string(),
                format!("PRAGMA busy_timeout = {};", busy_timeout_ms),
            ]
        }
        (
            DbKind::Postgres,
            DbSettings::Postgres {
                app_name,
                statement_timeout,
                idle_in_transaction_timeout,
                lock_timeout,
            },
        ) => {
            let mut stmts = vec![
                // application_name is safe to single-quote; minimal escaping
                format!("SET application_name = '{}';", app_name.replace('\'', "''")),
                "SET timezone = 'UTC';".to_string(),
                format!("SET statement_timeout = '{}';", statement_timeout),
                format!(
                    "SET idle_in_transaction_session_timeout = '{}';",
                    idle_in_transaction_timeout
                ),
            ];
            if let Some(lock_timeout) = lock_timeout {
                stmts.push(format!("SET lock_timeout = '{}';", lock_timeout));
            }
            stmts
        }
        _ => Vec::new(),
    }
}

/// Apply SQLite-specific per-connection settings
/// Extracts Sqlite variant from DbSettings and applies foreign_keys + busy_timeout
async fn apply_sqlite_config(
    conn: &mut sqlx::SqliteConnection,
    settings: &DbSettings,
) -> Result<(), sqlx::Error> {
    // Build and execute session statements for SQLite
    let statements = build_session_statements(DbKind::SqliteMemory, settings);
    for stmt in statements {
        sqlx::query(&stmt).execute(&mut *conn).await?;
    }
    Ok(())
}

/// Apply PostgreSQL-specific per-connection settings
/// Extracts Postgres variant from DbSettings and applies app_name, statement_timeout,
/// idle_in_transaction_timeout, and optionally lock_timeout
async fn apply_postgres_config(
    conn: &mut sqlx::PgConnection,
    settings: &DbSettings,
) -> Result<(), sqlx::Error> {
    // Build and execute session statements for Postgres
    let statements = build_session_statements(DbKind::Postgres, settings);
    for stmt in statements {
        sqlx::query(&stmt).execute(&mut *conn).await?;
    }
    Ok(())
}

/// Get SQLite lock file path for AI profile initialization (`<db>.ai_profiles.lock`)
fn sqlite_ai_profiles_lock_path(
    db_kind: DbKind,
    env: RuntimeEnv,
) -> Result<std::path::PathBuf, AppError> {
    use db_infra::sqlite_file_spec;

    match db_kind {
        DbKind::SqliteFile => {
            let file_spec = sqlite_file_spec(db_kind, env).map_err(AppError::from)?;
            Ok(std::path::Path::new(&file_spec).with_extension("ai_profiles.lock"))
        }
        _ => Err(AppError::config(
            "sqlite_ai_profiles_lock_path",
            crate::error::Sentinel("only works with SqliteFile database kind"),
        )),
    }
}

/// Fast-path check: verify if all required AI profiles already exist and match expected values
async fn fast_path_ai_profiles_check(conn: &DatabaseConnection) -> Result<bool, AppError> {
    use std::collections::HashSet;

    use crate::ai::registry;
    use crate::repos::ai_profiles::list_all;

    let existing_profiles = list_all(conn)
        .await
        .map_err(|e| AppError::config("failed to list AI profiles for fast-path check", e))?;

    // Build set of expected profile keys
    let mut expected_keys = HashSet::new();
    for factory in registry::registered_ais() {
        let key = (
            factory.name.to_string(),
            factory.version.to_string(),
            factory.profile.variant.to_string(),
        );
        expected_keys.insert(key);
    }

    // Build set of existing profile keys
    let existing_keys: HashSet<_> = existing_profiles
        .iter()
        .map(|p| {
            (
                p.registry_name.clone(),
                p.registry_version.clone(),
                p.variant.clone(),
            )
        })
        .collect();

    // Fast-path succeeds if all expected profiles exist
    Ok(expected_keys.is_subset(&existing_keys))
}

/// Ensure default AI profiles with database-level locking for coordination across processes
async fn ensure_ai_profiles_with_lock(
    pool: &DatabaseConnection, // admin_pool for Postgres/SqliteFile, shared_pool for SqliteMemory
    env: RuntimeEnv,
    db_kind: DbKind,
) -> Result<(), AppError> {
    // Fast-path: skip lock acquisition if profiles already exist
    if fast_path_ai_profiles_check(pool).await? {
        info!("ai_profiles_init=skipped up_to_date=true");
        return Ok(());
    }

    // Acquire lock with timeout and retry logic (same pattern as migrations)
    let lock_acquire_ms = std::env::var("NOMMIE_AI_PROFILES_INIT_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(match env {
            RuntimeEnv::Test => 3000,
            _ => 900,
        });

    let start = Instant::now();
    let mut attempts: u32 = 0;
    let guard = loop {
        attempts += 1;

        // Try to acquire lock based on database type
        let maybe_guard = match db_kind {
            DbKind::Postgres => {
                let url = make_conn_spec(env, db_kind, DbOwner::App).map_err(AppError::from)?;
                let sanitized_url = sanitize_db_url(&url);
                let key = format!("nommie:ai_profiles_init:{:?}:{}", db_kind, sanitized_url);
                let mut lock = PgAdvisoryLock::new(pool.clone(), &key);
                lock.try_acquire().await.map_err(AppError::from)?
            }
            DbKind::SqliteMemory => {
                let mut lock = InMemoryLock;
                lock.try_acquire().await.map_err(AppError::from)?
            }
            DbKind::SqliteFile => {
                let lock_path = sqlite_ai_profiles_lock_path(db_kind, env)?;
                let mut lock = SqliteFileLock::new(&lock_path).map_err(AppError::from)?;
                lock.try_acquire().await.map_err(AppError::from)?
            }
        };

        if let Some(acquired_guard) = maybe_guard {
            trace!(
                lock = "won",
                operation = "ai_profiles_init",
                env = ?env,
                db_kind = ?db_kind,
                attempts = attempts,
                elapsed_ms = start.elapsed().as_millis()
            );
            break acquired_guard;
        }

        let base_delay_ms = (5u64 << attempts.saturating_sub(1)).min(80);
        let jitter_ms = rand::rng().random::<u64>() % 4;
        let delay_ms = base_delay_ms + jitter_ms;

        trace!(
            lock = "backoff",
            operation = "ai_profiles_init",
            attempts = attempts,
            delay_ms = delay_ms,
            elapsed_ms = start.elapsed().as_millis()
        );

        tokio::time::sleep(Duration::from_millis(delay_ms)).await;

        if start.elapsed() >= Duration::from_millis(lock_acquire_ms) {
            let detail = format!(
                "AI profiles initialization lock acquisition timeout after {:?} ({} attempts)",
                start.elapsed(),
                attempts
            );
            return Err(AppError::config(
                detail,
                crate::error::Sentinel("ai_profiles_init_lock_timeout"),
            ));
        }
    };

    // Double-check after acquiring lock (another process might have initialized)
    if fast_path_ai_profiles_check(pool).await? {
        info!("ai_profiles_init=skipped up_to_date=true (after lock acquisition)");
        if let Err(release_err) = guard.release().await {
            warn!(error = %format!("{}", match release_err {
                DbInfraError::Config { message } => message,
            }), "Failed to release AI profiles initialization guard");
        }
        return Ok(());
    }

    // Actually initialize the profiles (while holding the lock)
    crate::repos::ai_profiles::ensure_default_ai_profiles(pool)
        .await
        .map_err(AppError::from)?;

    info!("ai_profiles_init=complete");

    // Release lock
    if let Err(release_err) = guard.release().await {
        warn!(error = %format!("{}", match release_err {
            DbInfraError::Config { message } => message,
        }), "Failed to release AI profiles initialization guard");
    }

    Ok(())
}

/// Determine database engine type for logging
/// Build the app DB *and* guarantee schema is current.
/// Uses unified migration orchestration with appropriate pool creation strategy.
///
/// FLOW:
/// - InMemory: Create shared pool → migrate on shared pool → init AI profiles on shared pool → return shared pool
/// - Others: Create admin pool → migrate on admin pool → init AI profiles on admin pool → create shared pool → return shared pool
///
/// INVARIANTS:
/// - InMemory: Must migrate on shared pool since each connection is its own database instance
/// - SqliteFile: Database-level PRAGMAs (journal_mode, synchronous) must be set before other connections exist
/// - Postgres: Migrations use admin pool for consistency; no technical requirement but maintains pattern
/// - Schema validation uses fast_path_schema_check to ensure migrations completed correctly
#[allow(clippy::explicit_auto_deref)]
pub async fn bootstrap_db(
    env: RuntimeEnv,
    db_kind: DbKind,
) -> Result<DatabaseConnection, AppError> {
    // Validate configuration first
    validate_db_config(env, db_kind)?;

    let engine = get_db_engine(db_kind);
    let path = get_db_path(db_kind);
    let pid = process::id();

    //  Bootstrap start marker - BEFORE any work
    info!(
        "bootstrap=start env={:?} db_kind={:?} engine={} path={} pid={}",
        env, db_kind, engine, path, pid
    );

    // Build migration pool (shared for SqliteMemory, admin for others)
    let migration_pool = match db_kind {
        DbKind::SqliteMemory => {
            // CRITICAL: For SQLite in-memory, we must migrate on the same connection
            // that will be returned, since each connection gets its own database instance
            get_or_create_shared_pool(env, db_kind).await?
        }
        _ => {
            // For non-in-memory databases, use admin pool for migrations
            // This ensures database-level settings (e.g., SQLite PRAGMAs) are set before other connections exist
            let admin_pool = db_infra::infra::db::core::build_admin_pool(env, db_kind).await?;
            Arc::new(admin_pool)
        }
    };

    // Common: run migrations and initialize AI profiles on the migration pool
    // For SqliteFile: Sets database-level PRAGMAs (journal_mode, synchronous) which require exclusive access
    db_infra::infra::db::core::orchestrate_migration_internal(
        &migration_pool,
        env,
        db_kind,
        MigrationCommand::Up,
    )
    .await?;

    ensure_ai_profiles_with_lock(&migration_pool, env, db_kind).await?;

    // Build final shared pool (reuse for SqliteMemory, create new for others)
    let shared_pool = match db_kind {
        DbKind::SqliteMemory => migration_pool, // reuse - same pool
        _ => {
            // Create shared pool AFTER migrations and admin operations complete
            // For SqliteFile: PRAGMAs are now set, so safe to create multiple connections
            get_or_create_shared_pool(env, db_kind).await?
        }
    };

    //  Bootstrap ready marker - JUST BEFORE returning the pool
    info!("bootstrap=ready");
    migration_counters::log_snapshot("bootstrap_db");

    // Return the shared pool (same connection that was migrated for InMemory, new pool for others)
    Ok((*shared_pool).clone())
}

/// Build admin pool for migrations - single connection only
/// This function is public for test utilities that need to query migration state
/// with admin-level access (e.g., counting applied migrations accurately).
pub async fn build_admin_pool(
    env: RuntimeEnv,
    db_kind: DbKind,
) -> Result<DatabaseConnection, AppError> {
    let url = make_conn_spec(env, db_kind, DbOwner::Owner)?;

    let mut opt = ConnectOptions::new(&url);
    opt.min_connections(1)
        .max_connections(1) // Admin pool uses exactly 1 connection
        .acquire_timeout(Duration::from_secs(2))
        .sqlx_logging(true);

    // Retry connection on startup for Postgres only (max 5 tries, 0.5s interval)
    // SQLite connections don't need retry since they're local
    let pool = if matches!(db_kind, DbKind::Postgres) {
        retry_connection(
            || {
                let opt_clone = opt.clone();
                async move {
                    Database::connect(opt_clone).await.map_err(|e| {
                        AppError::config("failed to connect to Postgres (admin pool)", e)
                    })
                }
            },
            5,
            500,
        )
        .await?
    } else {
        Database::connect(opt)
            .await
            .map_err(|e| AppError::config("failed to connect to database (admin pool)", e))?
    };
    Ok(pool)
}

pub async fn build_pool(
    env: RuntimeEnv,
    db_kind: DbKind,
    pool_cfg: &ConnectionSettings,
) -> Result<DatabaseConnection, AppError> {
    let url = make_conn_spec(env, db_kind, DbOwner::App)?;

    match db_kind {
        // ---------- SQLite (file and in-memory) ----------
        DbKind::SqliteFile | DbKind::SqliteMemory => {
            let connect_opts = SqliteConnectOptions::from_str(&url)
                .map_err(|e| AppError::config("invalid SQLite connection options", e))?
                .create_if_missing(true);

            // Build SQLx pool with per-connection PRAGMAs
            let db_settings = pool_cfg.db_settings.clone();
            let pool: SqlitePool = SqlitePoolOptions::new()
                .min_connections(pool_cfg.pool_min)
                .max_connections(pool_cfg.pool_max)
                .acquire_timeout(Duration::from_millis(pool_cfg.acquire_timeout_ms))
                .after_connect(move |conn, _meta| {
                    let settings = db_settings.clone();
                    Box::pin(async move {
                        apply_sqlite_config(conn, &settings).await?;
                        trace!("db=sqlite hook=after_connect ok");
                        Ok::<_, sqlx::Error>(())
                    })
                })
                .connect_with(connect_opts)
                .await
                .map_err(|e| AppError::config("failed to create SQLite connection pool", e))?;

            // warm-up to ensure hook ran on initial connection(s)
            if pool_cfg.pool_min > 0 {
                let mut conn = pool.acquire().await.map_err(|e| {
                    AppError::config("connection acquisition failed during warmup", e)
                })?;
                sqlx::query("SELECT 1;")
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| AppError::config("warmup query failed", e))?;
            }

            // Hand back to SeaORM
            let db = SqlxSqliteConnector::from_sqlx_sqlite_pool(pool);

            // Generate connection-based pool_id for consistent correlation
            let pool_id = sqlite_diagnostics::connection_id(&db);

            // Diagnostics snapshot
            sqlite_diagnostics::log_pragma_snapshot(&db, "shared").await?;

            info!(
                "pool=create engine=sqlite path={} pool_id={} min={} max={} acquire_timeout_ms={}",
                url, pool_id, pool_cfg.pool_min, pool_cfg.pool_max, pool_cfg.acquire_timeout_ms
            );
            Ok(db)
        }

        // ---------- Postgres (Prod/Test) ----------
        DbKind::Postgres => {
            info!(
                "pool=connecting engine=postgres url_configured={} min={} max={} acquire_timeout_ms={}",
                !url.is_empty(), pool_cfg.pool_min, pool_cfg.pool_max, pool_cfg.acquire_timeout_ms
            );

            // Build raw SQLx pool so we can apply a true per-connection hook
            // Note: No retry logic needed here - if we reach this point, the admin pool
            // connection already succeeded (with retries if needed), so the database is ready
            let db_settings = pool_cfg.db_settings.clone();
            let sqlx_pool = PgPoolOptions::new()
                .min_connections(pool_cfg.pool_min)
                .max_connections(pool_cfg.pool_max)
                .acquire_timeout(Duration::from_millis(pool_cfg.acquire_timeout_ms))
                .idle_timeout(Duration::from_secs(30))
                .after_connect(move |conn, _meta| {
                    let settings = db_settings.clone();
                    Box::pin(async move {
                        apply_postgres_config(conn, &settings).await?;
                        Ok::<_, sqlx::Error>(())
                    })
                })
                .connect(&url)
                .await
                .map_err(|e| AppError::config("failed to connect to Postgres", e))?;

            let db = SqlxPostgresConnector::from_sqlx_postgres_pool(sqlx_pool);

            // Generate connection-based pool_id for consistent correlation
            let pool_id = sqlite_diagnostics::connection_id(&db);

            info!(
                "pool=create engine=postgres path=postgres pool_id={} min={} max={} acquire_timeout_ms={}",
                pool_id, pool_cfg.pool_min, pool_cfg.pool_max, pool_cfg.acquire_timeout_ms
            );
            Ok(db)
        }
    }
}
