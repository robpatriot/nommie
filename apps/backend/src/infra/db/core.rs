// Standard library imports
use std::future::Future;
use std::process;
use std::str::FromStr;
use std::time::{Duration, Instant};

// External crate imports
use migration::{
    count_applied_migrations, get_latest_migration_version, migrate, MigrationCommand, Migrator,
    MigratorTrait,
};
use rand::Rng;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, DbErr,
    SqlxPostgresConnector, SqlxSqliteConnector, Statement,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

// Local module imports
use super::diagnostics::{migration_counters, sqlite_diagnostics};
use super::locking::{BootstrapLock, Guard, InMemoryLock, PgAdvisoryLock, SqliteFileLock};
// Use re-exported types from parent module
use super::{DbKind, DbOwner, RuntimeEnv};
// Internal crate imports
use crate::config::db::{
    build_connection_settings, make_conn_spec, sqlite_file_spec, validate_db_config,
    ConnectionSettings, DbSettings, PoolPurpose,
};
use crate::db::shared_pool_cache::get_or_create_shared_pool;
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::logging::pii::Redacted;

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

/// Helper function to create AppError with preserved error context
/// This provides better error context preservation than simple string formatting
pub fn config_error_with_context<E: std::error::Error + Send + Sync + 'static>(
    context: &str,
    source: E,
) -> AppError {
    AppError::config(context, source)
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

/// Apply database settings using SeaORM DatabaseConnection
/// This is used for migration/admin pools where the pool already exists.
/// For runtime pools, use apply_sqlite_config/apply_postgres_config with after_connect hooks.
async fn apply_db_settings(
    pool: &DatabaseConnection,
    settings: &DbSettings,
    db_kind: DbKind,
) -> Result<(), AppError> {
    let statements = build_session_statements(db_kind, settings);
    let backend = DatabaseBackend::from(db_kind);
    for stmt in statements {
        pool.execute(Statement::from_string(backend, stmt)).await?;
    }
    Ok(())
}

/// Determine database engine type for logging
/// Build the app DB *and* guarantee schema is current.
/// Uses unified migration orchestration with appropriate pool creation strategy.
///
/// FLOW:
/// - InMemory: Create shared pool → migrate on shared pool → validate schema → return shared pool
/// - Others: Create admin pool → migrate on admin pool → create shared pool → validate schema → return shared pool
///
/// INVARIANTS:
/// - InMemory: Must migrate on shared pool since each connection is its own database instance
/// - Others: Migrate on admin pool, then create separate shared pool for runtime use
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

    let shared_pool = match db_kind {
        DbKind::SqliteMemory => {
            // CRITICAL: For SQLite in-memory, we must migrate on the same connection
            // that will be returned, since each connection gets its own database instance

            // Create the shared pool first for in-memory databases
            let shared_pool = get_or_create_shared_pool(env, db_kind).await?;

            // Run migration orchestration on the shared pool for in-memory databases
            orchestrate_migration_internal(&shared_pool, env, db_kind, MigrationCommand::Up)
                .await?;

            shared_pool
        }
        _ => {
            // For non-in-memory databases, create admin pool, run migration, then create shared pool
            let admin_pool = build_admin_pool(env, db_kind).await?;

            // Run migration orchestration - handles all database types with locking and fast-path
            orchestrate_migration_internal(&admin_pool, env, db_kind, MigrationCommand::Up).await?;

            // CRITICAL INVARIANT: Shared pool is only created/reused AFTER migration and lock phases complete
            // The shared pool is never used for migration, locking, or admin operations
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
                        debug!("db=sqlite hook=after_connect ok");
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

/// Sanitize a database URL by removing password/secret components.
/// For postgresql://user:pass@host:port/db -> postgresql://user:***@host:port/db
fn sanitize_db_url(url: &str) -> String {
    // Simple regex-style replacement to hide passwords in database URLs
    // This handles the common pattern: scheme://user:password@host:port/database
    if url.contains("@") && url.contains(":") {
        let parts: Vec<&str> = url.split("@").collect();
        if parts.len() == 2 {
            let auth_part = parts[0];
            let host_part = parts[1];

            if let Some(colon_pos) = auth_part.rfind(':') {
                let scheme_user = &auth_part[..colon_pos];
                // Replace everything after the last colon in auth part with ***
                format!("{}:***@{}", scheme_user, host_part)
            } else {
                url.to_string()
            }
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    }
}

/// Fast-path schema check: compare current database schema state against expected.
/// Returns true if schema is up-to-date, false if migration is needed.
/// Autocommit + error-free if table is missing.
async fn fast_path_schema_check(conn: &DatabaseConnection) -> Result<bool, AppError> {
    migration_counters::schema_check();

    let expected_count = Migrator::migrations().len();

    // Get expected last migration version from the migrator
    let expected_last = Migrator::migrations()
        .last()
        .map(|m| m.name().to_string())
        .unwrap_or_default();

    // Try to get current migration state using the new migration functions
    let (current_count, current_last) = match count_applied_migrations(conn).await {
        Ok(count) => {
            debug!(
                fastpath = "debug",
                applied_count = count,
                expected_count = expected_count
            );
            // Migration table exists, get the latest version
            match get_latest_migration_version(conn).await {
                Ok(last) => {
                    debug!(fastpath = "debug", current_last = %last.as_deref().unwrap_or("None"), expected_last = %expected_last);
                    (count, last)
                }
                Err(e) => {
                    return Err(AppError::config(
                        "failed to get latest migration version",
                        e,
                    ));
                }
            }
        }
        Err(DbErr::Exec(_)) => {
            // Migration table doesn't exist yet
            debug!(fastpath = "miss", reason = "migration_table_missing");
            return Ok(false);
        }
        Err(e) => {
            return Err(AppError::config("failed to count applied migrations", e));
        }
    };

    // Compare current state against expected
    let is_up_to_date = current_count == expected_count
        && (!expected_last.is_empty() && current_last.as_deref() == Some(&expected_last));

    if is_up_to_date {
        migration_counters::fast_path_hit();
        debug!(
            fastpath = "hit",
            current_count = current_count,
            expected_count = expected_count,
            current_last = %Redacted(current_last.as_deref().unwrap_or("")),
            expected_last = %Redacted(&expected_last)
        );
    } else {
        migration_counters::fast_path_miss();
        debug!(
            fastpath = "miss",
            current_count = current_count,
            expected_count = expected_count,
            current_last = %Redacted(current_last.as_deref().unwrap_or("")),
            expected_last = %Redacted(&expected_last),
            reason = "count_or_version_mismatch"
        );
    }

    Ok(is_up_to_date)
}

// ============================================================================
// Migration Flow Functions
// ============================================================================

/// Orchestrate migration: builds admin pool and delegates to internal function.
/// Handles all database types (Postgres, SQLite file, InMemory).
pub async fn orchestrate_migration(
    env: RuntimeEnv,
    db_kind: DbKind,
    command: MigrationCommand,
) -> Result<(), AppError> {
    // Validate configuration first
    validate_db_config(env, db_kind)?;

    // Create admin pool for migration operations
    let admin_pool = build_admin_pool(env, db_kind).await?;

    // Delegate to internal function
    orchestrate_migration_internal(&admin_pool, env, db_kind, command).await
}

/// Orchestrate migration: uses provided pool and delegates to migrate_with_lock.
/// Handles all database types (Postgres, SQLite file, InMemory).
async fn orchestrate_migration_internal(
    pool: &DatabaseConnection,
    env: RuntimeEnv,
    db_kind: DbKind,
    command: MigrationCommand,
) -> Result<(), AppError> {
    // Create cancellation token for this migration
    let cancellation_token = CancellationToken::new();
    let engine = get_db_engine(db_kind);
    let path = get_db_path(db_kind);

    //  Migrate start marker - RIGHT BEFORE running migrations
    info!(
        "migrate=start env={:?} db_kind={:?} engine={} path={}",
        env, db_kind, engine, path
    );

    // Short-circuit for Status command - no lock, no fast-path check
    if matches!(command, MigrationCommand::Status) {
        migrate(pool, command)
            .await
            .map_err(|e| AppError::config("migration execution failed", e))?;
        info!("migrate=done");
        return Ok(());
    }

    // Create appropriate lock and run migration flow based on database type
    let url = make_conn_spec(env, db_kind, DbOwner::Owner)?;
    let result = match db_kind {
        DbKind::Postgres => {
            // Use the provided admin pool for PostgreSQL advisory lock
            // (pool is already the admin pool built by orchestrate_migration)
            let sanitized_url = sanitize_db_url(&url);
            let key = format!("nommie:migrate:{:?}:{}", db_kind, sanitized_url);

            let lock = PgAdvisoryLock::new(pool.clone(), &key);

            // Use admin pool for Postgres migrations (same pool/session used for advisory lock)
            migrate_with_lock(
                pool,
                lock,
                env,
                db_kind,
                command,
                cancellation_token.clone(),
            )
            .await
        }

        DbKind::SqliteMemory => {
            // Create no-op lock for InMemory
            let lock = InMemoryLock;

            migrate_with_lock(
                pool,
                lock,
                env,
                db_kind,
                command,
                cancellation_token.clone(),
            )
            .await
        }

        DbKind::SqliteFile => {
            // Create file lock
            // Get file spec for lock file
            let file_spec = sqlite_file_spec(db_kind, env)?;
            let lock_path = std::path::Path::new(&file_spec).with_extension("migrate.lock");
            let lock = SqliteFileLock::new(&lock_path)?;

            migrate_with_lock(
                pool,
                lock,
                env,
                db_kind,
                command,
                cancellation_token.clone(),
            )
            .await
        }
    };

    // Handle SQLITE_BUSY errors specifically
    if let Err(ref e) = result {
        let error_msg = e.to_string();
        if error_msg.contains("database is locked") || error_msg.contains("SQLITE_BUSY") {
            migration_counters::busy_event();
            error!("sqlite_busy op=migrate err={:?}", e);
        }
    }

    //  Migrate done marker - AFTER migration orchestration completes (success or error)
    info!("migrate=done");
    migration_counters::log_snapshot("migrate_orchestration");

    result
}

/// Core migration flow with lock acquisition and schema checks.
/// Features: Separated timeouts, single release point, task spawning, proper error mapping.
async fn migrate_with_lock<L>(
    pool: &DatabaseConnection,
    mut lock: L,
    env: RuntimeEnv,
    db_kind: DbKind,
    command: MigrationCommand,
    cancellation_token: tokio_util::sync::CancellationToken,
) -> Result<(), AppError>
where
    L: BootstrapLock,
{
    // Build migration connection settings outside of guard-controlled flow
    let connection_settings = build_connection_settings(env, db_kind, PoolPurpose::Migration)?;
    // Get timeout configurations with environment overrides
    let lock_acquire_ms = std::env::var("NOMMIE_MIGRATE_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(match env {
            RuntimeEnv::Test => 3000, // Test environment gets longer timeout
            _ => 900,                 // Other profiles
        });

    // Log timeout configuration once at start
    info!(
        acquire_ms = lock_acquire_ms,
        env = ?env,
        db_kind = ?db_kind,
        "migration timeouts configured"
    );

    let start = Instant::now();

    // Lock acquisition with backoff
    let mut attempts: u32 = 0;
    let guard = loop {
        attempts += 1;

        // Fast-path check before attempting lock acquisition (only for Up commands)
        if matches!(command, MigrationCommand::Up) && fast_path_schema_check(pool).await? {
            info!("migrate=skipped up_to_date=true");
            return Ok(());
        }

        // Try to acquire lock
        if let Some(acquired_guard) = lock.try_acquire().await? {
            migration_counters::add_lock_acquire_attempts(attempts as usize);
            migration_counters::lock_acquired();
            debug!(
                lock = "won",
                env = ?env,
                db_kind = ?db_kind,
                attempts = attempts,
                elapsed_ms = start.elapsed().as_millis()
            );
            break acquired_guard;
        }

        // Lock acquisition failed - backoff with existing jitter
        let base_delay_ms = (5u64 << attempts.saturating_sub(1)).min(80);
        let jitter_ms = rand::rng().random::<u64>() % 4; // BACKOFF_JITTER_MS_MAX: 0-3
        let delay_ms = base_delay_ms + jitter_ms;

        debug!(
            lock = "backoff",
            attempts = attempts,
            delay_ms = delay_ms,
            elapsed_ms = start.elapsed().as_millis()
        );
        migration_counters::lock_backoff_event();

        // Backoff with cancellation check
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {
                // Check acquire timeout after sleep
                if start.elapsed() >= Duration::from_millis(lock_acquire_ms) {
                    migration_counters::lock_acquire_timeout();
                    return Err(AppError::timeout(
                        ErrorCode::LockTimeoutAcquire,
                        "migration lock acquisition timeout",
                        std::io::Error::other(format!(
                            "lock acquisition timed out after {:?} ({} attempts)",
                            start.elapsed(), attempts
                        )),
                    ));
                }
            }
            _ = cancellation_token.cancelled() => {
                info!(
                    elapsed_ms = start.elapsed().as_millis(),
                    attempts = attempts,
                    "Migration cancelled during acquire backoff"
                );
                return Err(AppError::internal(
                    ErrorCode::MigrationCancelled,
                    "migration cancelled during acquire",
                    std::io::Error::other(format!(
                        "migration cancelled during acquire backoff after {}ms",
                        start.elapsed().as_millis()
                    )),
                ));
            }
        }

        // Continue to next attempt
    };

    // Execute migration with guard (single release point from here)
    let result = migrate_with_guard_controlled(
        pool,
        guard,
        env,
        db_kind,
        command,
        cancellation_token,
        connection_settings.db_settings.clone(),
    )
    .await;

    // SINGLE RELEASE POINT: Always release guard here, regardless of outcome
    match result {
        Ok(guard_back) => {
            if let Err(release_err) = guard_back.release().await {
                warn!(
                    error = %release_err,
                    "Failed to release migration guard"
                );
            }
            Ok(())
        }
        Err((guard_back, migration_err)) => {
            if let Err(release_err) = guard_back.release().await {
                warn!(
                    error = %release_err,
                    "Failed to release migration guard after error"
                );
            }
            Err(migration_err)
        }
    }
}

/// Controlled migration execution with task spawning and timeout.
/// Returns Result<Guard, (Guard, AppError)> to ensure single release point.
async fn migrate_with_guard_controlled(
    pool: &DatabaseConnection,
    guard: Guard,
    env: RuntimeEnv,
    db_kind: DbKind,
    command: MigrationCommand,
    cancellation_token: CancellationToken,
    db_settings: DbSettings,
) -> Result<Guard, (Guard, AppError)> {
    let start = Instant::now();
    let body_timeout_ms: u64 = 120000;

    // SQLite file prerequisites: journal_mode and synchronous (before general settings)
    if matches!(db_kind, DbKind::SqliteFile) {
        if let Err(e) = setup_sqlite_file_prerequisites(pool).await {
            return Err((guard, e));
        }
    }

    // Apply unified database settings using pre-built settings
    if let Err(e) = apply_db_settings(pool, &db_settings, db_kind).await {
        return Err((guard, e));
    }

    // Spawn migration task for controlled execution
    let pool_clone = pool.clone();
    let command_clone = command.clone();
    let mut migration_task = tokio::spawn(async move { migrate(&pool_clone, command_clone).await });

    // tokio::select! between task completion, timeout, and cancellation
    let migration_result = tokio::select! {
        biased; // Prioritize task completion over timeout when both are ready

        // Task completion branch
        task_result = &mut migration_task => {
            match task_result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => { migration_counters::migration_failed(); Err(AppError::internal(
                    ErrorCode::MigrationFailed,
                    "migration execution failed",
                    e,
                )) },
                Err(join_err) => {
                    migration_counters::migration_failed();
                    if join_err.is_panic() {
                        Err(AppError::internal_msg(
                            ErrorCode::MigrationFailed,
                            "migration task panicked",
                            "migration task panicked during execution",
                        ))
                    } else {
                        Err(AppError::internal_msg(
                            ErrorCode::MigrationFailed,
                            "migration task was aborted",
                            "migration task was aborted before completion",
                        ))
                    }
                }
            }
        }
        // Timeout branch - abort task and await its termination before returning
        _ = tokio::time::sleep(Duration::from_millis(body_timeout_ms)) => {
            migration_task.abort();
            let _ = migration_task.await; // Await termination before returning
            info!(
                elapsed_ms = start.elapsed().as_millis(),
                "Migration body timeout - task aborted"
            );
            migration_counters::migration_body_timeout();
            Err(AppError::timeout(
                ErrorCode::LockTimeoutBody,
                "migration body timeout",
                std::io::Error::other(format!("migration body execution timed out after {}ms", body_timeout_ms)),
            ))
        }
        // Cancellation branch - abort task and await its termination before returning
        _ = cancellation_token.cancelled() => {
            migration_task.abort();
            let _ = migration_task.await; // Await termination before returning
            info!(
                elapsed_ms = start.elapsed().as_millis(),
                "Migration cancelled during body execution - task aborted"
            );
            migration_counters::migration_cancelled();
            Err(AppError::internal(
                ErrorCode::MigrationCancelled,
                "migration cancelled during execution",
                std::io::Error::other(format!(
                    "migration cancelled during body execution after {}ms",
                    start.elapsed().as_millis()
                )),
            ))
        }
    };

    // Handle migration result
    if let Err(e) = migration_result {
        return Err((guard, e));
    }

    migration_counters::migrator_ran();
    info!(migrator = "ran", env = ?env, db_kind = ?db_kind, elapsed_ms = start.elapsed().as_millis());

    // Post-check: Verify migrations match expected state based on command (still under lock)
    let expected_count = Migrator::migrations().len();
    let applied_count = count_applied_migrations(pool).await.unwrap_or(0);
    info!(
        migrate = "counts",
        expected_count = expected_count,
        applied_count = applied_count
    );

    // Verify migrations match expected state based on command type
    match command {
        MigrationCommand::Reset => {
            // Reset should rollback all migrations, leaving 0 applied
            if applied_count != 0 {
                migration_counters::postcheck_mismatch();
                let detail = format!(
                    "Migration verification failed: reset should leave 0 migrations applied, but {} were found (env={:?}, db_kind={:?})",
                    applied_count, env, db_kind
                );
                return Err((
                    guard,
                    AppError::internal(
                        ErrorCode::PostcheckMismatch,
                        detail,
                        crate::error::Sentinel("migration verification postcheck mismatch"),
                    ),
                ));
            }
        }
        MigrationCommand::Down => {
            // Down rolls back migrations, so we can't verify exact count without knowing steps
            // Just log the result - verification would require tracking before/after state
            info!(
                migrate = "down_complete",
                applied_count = applied_count,
                expected_count = expected_count
            );
        }
        MigrationCommand::Up | MigrationCommand::Fresh | MigrationCommand::Refresh => {
            // These commands should result in all migrations being applied
            if applied_count != expected_count {
                migration_counters::postcheck_mismatch();
                let detail = format!(
                    "Migration verification failed: expected {} migrations, but {} were applied (env={:?}, db_kind={:?})",
                    expected_count, applied_count, env, db_kind
                );
                return Err((
                    guard,
                    AppError::internal(
                        ErrorCode::PostcheckMismatch,
                        detail,
                        crate::error::Sentinel("migration verification postcheck mismatch"),
                    ),
                ));
            }
        }
        MigrationCommand::Status => {
            // Status doesn't change state, no verification needed
            // (This shouldn't reach here as Status is short-circuited earlier, but included for completeness)
        }
    }

    // Return guard for single release point
    Ok(guard)
}

/// Setup SQLite file prerequisites: journal_mode and synchronous
/// These must be set after acquiring the migration lock and before applying general settings
async fn setup_sqlite_file_prerequisites(pool: &DatabaseConnection) -> Result<(), AppError> {
    pool.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA journal_mode = WAL;",
    ))
    .await?;

    pool.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA synchronous = NORMAL;",
    ))
    .await?;

    Ok(())
}
