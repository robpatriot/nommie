use std::sync::Arc;
use std::time::{Duration, Instant};

use migration::{migrate, MigrationCommand, Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr};
use tokio_util::sync::CancellationToken;
use tracing::{info, trace, warn};

use crate::config::db::{
    build_connection_settings, build_session_statements, make_conn_spec, sanitize_db_url,
    DbOwner, DbSettings, PoolPurpose, RuntimeEnv,
};
use crate::error::DbInfraError;
use crate::infra::db::advisory_lock::{acquire_bootstrap_lock, AcquireResult, LockCallbacks};
use crate::infra::db::diagnostics::migration_counters;
use crate::infra::db::locking::Guard;

fn get_db_path(env: RuntimeEnv) -> String {
    let spec = make_conn_spec(env, DbOwner::Owner)
        .unwrap_or_else(|_| "postgres-unknown".to_string());
    sanitize_db_url(&spec)
}

pub async fn build_admin_pool(env: RuntimeEnv) -> Result<DatabaseConnection, DbInfraError> {
    let url = make_conn_spec(env, DbOwner::Owner)?;

    let mut opt = ConnectOptions::new(&url);
    opt.min_connections(1)
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(2))
        .sqlx_logging(true);

    let pool = Database::connect(opt)
        .await
        .map_err(|e| DbInfraError::Config {
            message: format!("failed to connect to database (admin pool): {e}"),
        })?;

    Ok(pool)
}

async fn fast_path_schema_check(conn: &DatabaseConnection) -> Result<bool, DbInfraError> {
    migration_counters::schema_check();

    let expected_count = Migrator::migrations().len();

    let expected_last = Migrator::migrations()
        .last()
        .map(|m| m.name().to_string())
        .unwrap_or_default();

    let (current_count, current_last) = match Migrator::get_applied_migrations(conn).await {
        Ok(migrations) => {
            let count = migrations.len();
            let last = migrations.last().map(|m| m.name().to_string());
            trace!(
                fastpath = "debug",
                applied_count = count,
                expected_count = expected_count,
                current_last = %last.as_deref().unwrap_or("None"),
                expected_last = %expected_last
            );
            (count, last)
        }
        Err(DbErr::Exec(_)) => {
            trace!(fastpath = "miss", reason = "migration_table_missing");
            return Ok(false);
        }
        Err(e) => {
            return Err(DbInfraError::Config {
                message: format!("failed to get applied migrations: {}", e),
            });
        }
    };

    let is_up_to_date = current_count == expected_count
        && (!expected_last.is_empty() && current_last.as_deref() == Some(&expected_last));

    if is_up_to_date {
        migration_counters::fast_path_hit();
    } else {
        migration_counters::fast_path_miss();
    }

    Ok(is_up_to_date)
}

pub async fn orchestrate_migration(
    env: RuntimeEnv,
    command: MigrationCommand,
) -> Result<(), DbInfraError> {
    let admin_pool = build_admin_pool(env).await?;

    let res = orchestrate_migration_internal(&admin_pool, env, command).await;
    res
}

pub async fn orchestrate_migration_internal(
    pool: &DatabaseConnection,
    env: RuntimeEnv,
    command: MigrationCommand,
) -> Result<(), DbInfraError> {
    let cancellation_token = CancellationToken::new();
    let path = get_db_path(env);

    info!(
        "migrate=start env={:?} engine=postgresql path={}",
        env, path
    );

    if matches!(command, MigrationCommand::Status) {
        migrate(pool, command)
            .await
            .map_err(|e| DbInfraError::Config {
                message: format!("migration execution failed: {}", e),
            })?;
        info!("migrate=done");
        return Ok(());
    }

    let result = migrate_with_lock(pool, env, command, cancellation_token.clone()).await;

    info!("migrate=done");
    migration_counters::log_snapshot("migrate_orchestration");

    result
}

async fn migrate_with_lock(
    pool: &DatabaseConnection,
    env: RuntimeEnv,
    command: MigrationCommand,
    cancellation_token: CancellationToken,
) -> Result<(), DbInfraError> {
    let connection_settings = build_connection_settings(env, PoolPurpose::Migration)?;

    let callbacks = LockCallbacks {
        on_acquired: Some(Arc::new(|_| migration_counters::lock_acquired())),
        on_backoff: Some(Arc::new(migration_counters::lock_backoff_event)),
        on_timeout: Some(Arc::new(migration_counters::lock_acquire_timeout)),
        on_add_attempts: Some(Arc::new(migration_counters::add_lock_acquire_attempts)),
    };

    let fast_path = matches!(command, MigrationCommand::Up).then(|| {
        let pool = pool.clone();
        move || {
            let pool = pool.clone();
            async move { fast_path_schema_check(&pool).await }
        }
    });

    let acquire_result = acquire_bootstrap_lock(
        pool,
        env,
        fast_path,
        Some(cancellation_token.clone()),
        callbacks,
    )
    .await?;

    let guard = match acquire_result {
        AcquireResult::Skipped => {
            info!("migrate=skipped up_to_date=true");
            return Ok(());
        }
        AcquireResult::Acquired(g) => g,
    };

    let result = migrate_with_guard_controlled(
        pool,
        guard,
        env,
        command,
        cancellation_token,
        connection_settings.db_settings.clone(),
    )
    .await;

    match result {
        Ok(guard_back) => {
            if let Err(release_err) = guard_back.release().await {
                warn!(error = %format!("{}", match release_err {
                    DbInfraError::Config { message } => message,
                }), "Failed to release migration guard");
            }
            Ok(())
        }
        Err((guard_back, migration_err)) => {
            if let Err(release_err) = guard_back.release().await {
                warn!(error = %format!("{}", match release_err {
                    DbInfraError::Config { message } => message,
                }), "Failed to release migration guard after error");
            }
            Err(migration_err)
        }
    }
}

async fn migrate_with_guard_controlled(
    pool: &DatabaseConnection,
    guard: Guard,
    env: RuntimeEnv,
    command: MigrationCommand,
    cancellation_token: CancellationToken,
    db_settings: DbSettings,
) -> Result<Guard, (Guard, DbInfraError)> {
    let start = Instant::now();
    let body_timeout_ms: u64 = 120000;

    if let Err(e) = apply_db_settings(pool, &db_settings).await {
        return Err((guard, e));
    }

    let pool_clone = pool.clone();
    let command_clone = command.clone();
    let mut migration_task = tokio::spawn(async move { migrate(&pool_clone, command_clone).await });

    let migration_result = tokio::select! {
        biased;

        task_result = &mut migration_task => {
            match task_result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => {
                    migration_counters::migration_failed();
                    Err(DbInfraError::Config {
                        message: format!("migration execution failed: {}", e),
                    })
                }
                Err(join_err) => {
                    migration_counters::migration_failed();
                    if join_err.is_panic() {
                        Err(DbInfraError::Config {
                            message: "migration task panicked during execution".to_string(),
                        })
                    } else {
                        Err(DbInfraError::Config {
                            message: "migration task was aborted before completion".to_string(),
                        })
                    }
                }
            }
        }
        _ = tokio::time::sleep(Duration::from_millis(body_timeout_ms)) => {
            migration_task.abort();
            let _ = migration_task.await;
            info!(
                elapsed_ms = start.elapsed().as_millis(),
                "Migration body timeout - task aborted"
            );
            migration_counters::migration_body_timeout();
            Err(DbInfraError::Config {
                message: format!(
                    "migration body execution timed out after {}ms",
                    body_timeout_ms
                ),
            })
        }
        _ = cancellation_token.cancelled() => {
            migration_task.abort();
            let _ = migration_task.await;
            info!(
                elapsed_ms = start.elapsed().as_millis(),
                "Migration cancelled during body execution - task aborted"
            );
            migration_counters::migration_cancelled();
            Err(DbInfraError::Config {
                message: format!(
                    "migration cancelled during body execution after {}ms",
                    start.elapsed().as_millis()
                ),
            })
        }
    };

    if let Err(e) = migration_result {
        return Err((guard, e));
    }

    migration_counters::migrator_ran();
    info!(
        migrator = "ran",
        env = ?env,
        elapsed_ms = start.elapsed().as_millis()
    );

    let expected_count = Migrator::migrations().len();
    let applied_count = match Migrator::get_applied_migrations(pool).await {
        Ok(migrations) => migrations.len(),
        Err(_) => 0,
    };
    info!(
        migrate = "counts",
        expected_count = expected_count,
        applied_count = applied_count
    );

    match command {
        MigrationCommand::Reset => {
            if applied_count != 0 {
                migration_counters::postcheck_mismatch();
                let detail = format!(
                    "Migration verification failed: reset should leave 0 migrations applied, but {} were found (env={:?})",
                    applied_count, env
                );
                return Err((guard, DbInfraError::Config { message: detail }));
            }
        }
        MigrationCommand::Down => {
            info!(
                migrate = "down_complete",
                applied_count = applied_count,
                expected_count = expected_count
            );
        }
        MigrationCommand::Up | MigrationCommand::Fresh | MigrationCommand::Refresh => {
            if applied_count != expected_count {
                migration_counters::postcheck_mismatch();
                let detail = format!(
                    "Migration verification failed: expected {} migrations, but {} were applied (env={:?})",
                    expected_count, applied_count, env
                );
                return Err((guard, DbInfraError::Config { message: detail }));
            }
        }
        MigrationCommand::Status => {}
    }

    Ok(guard)
}

async fn apply_db_settings(
    pool: &DatabaseConnection,
    settings: &DbSettings,
) -> Result<(), DbInfraError> {
    let statements = build_session_statements(settings);
    let backend = sea_orm::DatabaseBackend::Postgres;
    for stmt in statements {
        pool.execute(sea_orm::Statement::from_string(backend, stmt))
            .await
            .map_err(|e| DbInfraError::Config {
                message: format!("failed to apply db settings: {}", e),
            })?;
    }
    Ok(())
}
