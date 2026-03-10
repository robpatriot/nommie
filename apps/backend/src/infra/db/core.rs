use std::process;
use std::sync::Arc;
use std::time::Duration;

use db_infra::config::db::build_session_statements;
use db_infra::error::DbInfraError;
use db_infra::infra::db::advisory_lock::{acquire_bootstrap_lock, AcquireResult};
use db_infra::infra::db::diagnostics::{db_diagnostics, migration_counters};
use migration::MigrationCommand;
use sea_orm::{DatabaseConnection, SqlxPostgresConnector};
use sqlx::postgres::PgPoolOptions;
use tracing::{info, warn};

use super::{DbOwner, RuntimeEnv};
use crate::config::db::{make_conn_spec, ConnectionSettings, DbSettings};
use crate::db::shared_pool_cache::get_or_create_shared_pool;
use crate::error::AppError;

async fn apply_postgres_config(
    conn: &mut sqlx::PgConnection,
    settings: &DbSettings,
) -> Result<(), sqlx::Error> {
    let statements = build_session_statements(settings);
    for stmt in statements {
        sqlx::query(&stmt).execute(&mut *conn).await?;
    }
    Ok(())
}

async fn fast_path_ai_profiles_check(conn: &DatabaseConnection) -> Result<bool, AppError> {
    use std::collections::HashMap;

    use crate::ai::registry;
    use crate::repos::ai_profiles::{list_all, profile_matches_defaults};

    let existing_profiles = list_all(conn)
        .await
        .map_err(|e| AppError::config("failed to list AI profiles for fast-path check", e))?;

    let mut profile_map: HashMap<(String, String, String), crate::repos::ai_profiles::AiProfile> =
        HashMap::new();
    for profile in existing_profiles {
        let key = (
            profile.registry_name.clone(),
            profile.registry_version.clone(),
            profile.variant.clone(),
        );
        profile_map.insert(key, profile);
    }

    for factory in registry::registered_ais() {
        let profile_defaults = &factory.profile;
        let key = (
            factory.name.to_string(),
            factory.version.to_string(),
            profile_defaults.variant.to_string(),
        );

        match profile_map.get(&key) {
            Some(existing) => {
                if !profile_matches_defaults(existing, profile_defaults) {
                    return Ok(false);
                }
            }
            None => return Ok(false),
        }
    }

    Ok(true)
}

async fn ensure_ai_profiles_with_lock(
    pool: &DatabaseConnection,
    env: RuntimeEnv,
) -> Result<(), AppError> {
    let pool_clone = pool.clone();
    let fast_path = move || {
        let pool = pool_clone.clone();
        async move { fast_path_ai_profiles_check(&pool).await }
    };

    let acquire_result = acquire_bootstrap_lock(
        pool,
        env,
        Some(fast_path),
        None,
        db_infra::infra::db::advisory_lock::LockCallbacks::default(),
    )
    .await?;

    let guard = match acquire_result {
        AcquireResult::Skipped => {
            info!("ai_profiles_init=skipped up_to_date=true");
            return Ok(());
        }
        AcquireResult::Acquired(g) => g,
    };

    if fast_path_ai_profiles_check(pool).await? {
        info!("ai_profiles_init=skipped up_to_date=true (after lock acquisition)");
        if let Err(release_err) = guard.release().await {
            warn!(error = %format!("{}", match release_err {
                DbInfraError::Config { message } => message,
            }), "Failed to release AI profiles initialization guard");
        }
        return Ok(());
    }

    crate::repos::ai_profiles::ensure_default_ai_profiles(pool)
        .await
        .map_err(AppError::from)?;

    info!("ai_profiles_init=complete");

    if let Err(release_err) = guard.release().await {
        warn!(error = %format!("{}", match release_err {
            DbInfraError::Config { message } => message,
        }), "Failed to release AI profiles initialization guard");
    }

    Ok(())
}

async fn seed_admission_from_env(pool: &DatabaseConnection) -> Result<(), AppError> {
    use sea_orm::TransactionTrait;

    let patterns = crate::repos::allowed_emails::parse_allowed_emails_from_env();
    let admin_emails = crate::repos::allowed_emails::parse_admin_emails_from_env();
    if patterns.is_empty() && admin_emails.is_empty() {
        return Ok(());
    }

    let txn = pool
        .begin()
        .await
        .map_err(|e| AppError::config("failed to begin transaction for admission seed", e))?;

    let mut inserted = 0;
    if !patterns.is_empty() {
        inserted = crate::repos::allowed_emails::seed_from_env(&txn)
            .await
            .map_err(AppError::from)?;
    }

    let mut admin_count = 0;
    if !admin_emails.is_empty() {
        admin_count = crate::repos::allowed_emails::seed_admin_from_env(&txn)
            .await
            .map_err(AppError::from)?;
    }

    txn.commit()
        .await
        .map_err(|e| AppError::config("failed to commit admission seed", e))?;

    if inserted > 0 {
        info!(inserted, "admission_seed=complete");
    }
    if admin_count > 0 {
        info!(admin_count, "admin_seed=complete");
    }

    Ok(())
}

pub async fn bootstrap_db(env: RuntimeEnv) -> Result<DatabaseConnection, AppError> {
    let pid = process::id();

    tracing::debug!("bootstrap=start env={:?} pid={}", env, pid);

    let admin_pool = db_infra::infra::db::core::build_admin_pool(env).await?;
    let migration_pool = Arc::new(admin_pool);

    db_infra::infra::db::core::orchestrate_migration_internal(
        &migration_pool,
        env,
        MigrationCommand::Up,
    )
    .await?;

    ensure_ai_profiles_with_lock(&migration_pool, env).await?;

    seed_admission_from_env(&migration_pool).await?;

    let shared_pool = get_or_create_shared_pool(env).await?;

    info!("bootstrap=ready");
    migration_counters::log_snapshot("bootstrap_db");

    Ok(shared_pool.as_ref().clone())
}

pub async fn build_pool(
    env: RuntimeEnv,
    pool_cfg: &ConnectionSettings,
) -> Result<DatabaseConnection, AppError> {
    let url = make_conn_spec(env, DbOwner::App)?;

    info!(
        "pool=connecting engine=postgres url_configured={} min={} max={} acquire_timeout_ms={}",
        !url.is_empty(),
        pool_cfg.pool_min,
        pool_cfg.pool_max,
        pool_cfg.acquire_timeout_ms
    );

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

    let pool_id = db_diagnostics::connection_id(&db);

    info!(
        "pool=create engine=postgres path=postgres pool_id={} min={} max={} acquire_timeout_ms={}",
        pool_id, pool_cfg.pool_min, pool_cfg.pool_max, pool_cfg.acquire_timeout_ms
    );
    Ok(db)
}
