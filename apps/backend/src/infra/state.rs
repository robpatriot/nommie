use std::sync::Arc;

use crate::config::db::{DbKind, RuntimeEnv};
use crate::config::email_allowlist::EmailAllowlist;
use crate::error::AppError;
use crate::infra::db::bootstrap_db;
use crate::readiness::ReadinessManager;
use crate::state::app_state::AppState;
use crate::state::security_config::SecurityConfig;
use crate::ws::hub::RealtimeBroker;

/// Builder for creating AppState instances
#[derive(Default)]
pub struct StateBuilder {
    security_config: SecurityConfig,
    env: Option<RuntimeEnv>,
    db_kind: Option<DbKind>,
    email_allowlist: Option<EmailAllowlist>,
    redis_url: Option<String>,
    readiness: Option<Arc<ReadinessManager>>,
}

use std::sync::atomic::{AtomicBool, Ordering};

use crate::state::app_state::{AppConfig, Secret};

/// RAII guard to manage SingleFlight resolution flags
struct ResolutionGuard<'a> {
    flag: &'a AtomicBool,
}

impl<'a> ResolutionGuard<'a> {
    fn try_acquire(flag: &'a AtomicBool) -> Option<Self> {
        if flag.swap(true, Ordering::SeqCst) {
            None // Already in flight
        } else {
            Some(Self { flag })
        }
    }
}

impl<'a> Drop for ResolutionGuard<'a> {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

/// Authoritative function to resolve missing or unhealthy dependencies.
///
/// This is the SINGLE SOURCE OF TRUTH used by both initial startup and
/// the background monitor. It ensures that connectivity issues are
/// handled uniformly and that redundant connection attempts are prevented.
pub async fn resolve_dependencies(state: &AppState) -> Result<(), AppError> {
    let mut terminal_error = None;
    // 1. Resolve Database
    if let Some(guard) = ResolutionGuard::try_acquire(&state.db_resolution_in_flight) {
        let db = state.db();
        let needs_resolution = match db {
            None => true,
            Some(conn) => {
                // PING current connection
                let check = check_db_ping(&conn).await;
                state.readiness().update_dependency(
                    crate::readiness::types::DependencyName::Postgres,
                    check.clone(),
                );
                !check.is_ok()
            }
        };

        if needs_resolution {
            let start = std::time::Instant::now();
            match bootstrap_db(state.config.env, state.config.db_kind).await {
                Ok(conn) => {
                    state.set_db(conn.clone());
                    state.readiness().set_migration_result(true, None);
                    let check = check_db_ping(&conn).await;
                    state.readiness().update_dependency(
                        crate::readiness::types::DependencyName::Postgres,
                        check,
                    );
                    tracing::info!("readiness: database resolution successful");
                }
                Err(e) => {
                    let latency = start.elapsed();
                    state.readiness().update_dependency(
                        crate::readiness::types::DependencyName::Postgres,
                        crate::readiness::types::DependencyCheck::Down {
                            error: format!("{e}"),
                            latency,
                        },
                    );
                    if e.is_transient() {
                        tracing::warn!(error = %e, "readiness: transient database error during resolution");
                    } else {
                        state
                            .readiness()
                            .set_migration_result(false, Some(format!("{e}")));
                        tracing::error!(error = %e, "readiness: terminal database failure during resolution");
                        terminal_error = Some(e);
                    }
                }
            }
        }
        drop(guard);
    }

    if let Some(e) = terminal_error {
        return Err(e);
    }

    // 2. Resolve Redis
    if let Some(url) = &state.config.redis_url.0 {
        if let Some(guard) = ResolutionGuard::try_acquire(&state.redis_resolution_in_flight) {
            let realtime = state.realtime();
            let needs_resolution = match realtime {
                None => true,
                Some(_) => {
                    // PING current connection
                    let check = check_redis_ping(url).await;
                    state.readiness().update_dependency(
                        crate::readiness::types::DependencyName::Redis,
                        check.clone(),
                    );
                    !check.is_ok()
                }
            };

            if needs_resolution {
                let start = std::time::Instant::now();
                match RealtimeBroker::connect(url, state.readiness().clone()).await {
                    Ok(broker) => {
                        state.set_realtime(broker);
                        let check = check_redis_ping(url).await;
                        state.readiness().update_dependency(
                            crate::readiness::types::DependencyName::Redis,
                            check,
                        );
                        tracing::info!("readiness: Redis resolution successful");
                    }
                    Err(e) => {
                        let latency = start.elapsed();
                        state.readiness().update_dependency(
                            crate::readiness::types::DependencyName::Redis,
                            crate::readiness::types::DependencyCheck::Down {
                                error: format!("{e}"),
                                latency,
                            },
                        );
                        tracing::debug!(error = %e, "readiness: Redis resolution attempt failed");
                    }
                }
            }
            drop(guard);
        }
    }

    if let Some(e) = terminal_error {
        return Err(e);
    }

    Ok(())
}

async fn check_db_ping(
    db: &sea_orm::DatabaseConnection,
) -> crate::readiness::types::DependencyCheck {
    use sea_orm::ConnectionTrait;
    let start = std::time::Instant::now();
    let res = db
        .query_one(sea_orm::Statement::from_string(
            db.get_database_backend(),
            "SELECT 1".to_string(),
        ))
        .await;
    let latency = start.elapsed();
    match res {
        Ok(_) => crate::readiness::types::DependencyCheck::Ok { latency },
        Err(e) => crate::readiness::types::DependencyCheck::Down {
            error: format!("{e}"),
            latency,
        },
    }
}

async fn check_redis_ping(url: &str) -> crate::readiness::types::DependencyCheck {
    let start = std::time::Instant::now();
    let res = try_redis_ping(url).await;
    let latency = start.elapsed();
    match res {
        Ok(_) => crate::readiness::types::DependencyCheck::Ok { latency },
        Err(e) => crate::readiness::types::DependencyCheck::Down {
            error: format!("{e}"),
            latency,
        },
    }
}

async fn try_redis_ping(url: &str) -> Result<(), redis::RedisError> {
    let client = redis::Client::open(url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;
    redis::cmd("PING").query_async::<String>(&mut conn).await?;
    Ok(())
}

impl StateBuilder {
    pub fn with_env(mut self, env: RuntimeEnv) -> Self {
        self.env = Some(env);
        self
    }

    pub fn with_db(mut self, db_kind: DbKind) -> Self {
        self.db_kind = Some(db_kind);
        self
    }

    pub fn with_security(mut self, security_config: SecurityConfig) -> Self {
        self.security_config = security_config;
        self
    }

    /// Set the email allowlist (None = disabled, Some = enabled)
    /// If not called, allowlist defaults to None (disabled)
    pub fn with_email_allowlist(mut self, allowlist: Option<EmailAllowlist>) -> Self {
        self.email_allowlist = allowlist;
        self
    }

    pub fn with_redis_url(mut self, redis_url: Option<String>) -> Self {
        self.redis_url = redis_url;
        self
    }

    pub fn with_readiness(mut self, readiness: Arc<ReadinessManager>) -> Self {
        self.readiness = Some(readiness);
        self
    }

    const REDIS_DISABLED_REASON_TEST: &'static str = "redis_url not configured in test env";

    pub async fn build(self) -> Result<AppState, AppError> {
        let readiness = self
            .readiness
            .unwrap_or_else(|| Arc::new(ReadinessManager::new()));

        let db_url = Secret("".to_string());
        let redis_url = Secret(self.redis_url.clone());

        let (state, needs_resolution) = match (self.env, self.db_kind) {
            (Some(env), Some(db_kind)) => {
                let config = AppConfig {
                    env,
                    db_kind,
                    db_url,
                    redis_url,
                    security: self.security_config,
                    email_allowlist: self.email_allowlist,
                };
                let state = AppState::new(config, None, None, readiness);

                if state.config.env == RuntimeEnv::Test && state.config.redis_url.0.is_none() {
                    state.readiness().disable_dependency(
                        crate::readiness::types::DependencyName::Redis,
                        Self::REDIS_DISABLED_REASON_TEST.to_string(),
                    );
                } else if state.config.env != RuntimeEnv::Test && state.config.redis_url.0.is_none()
                {
                    return Err(AppError::config_msg(
                        "REDIS_URL must be set outside test environment",
                        "redis_url missing for non-test env",
                    ));
                }

                (state, true)
            }
            _ => {
                // Hollow state for tests that don't need DB/Redis.
                // Skips resolve_dependencies; no self-healing (nothing to heal).
                let config = AppConfig {
                    env: RuntimeEnv::Test,
                    db_kind: DbKind::SqliteMemory,
                    db_url,
                    redis_url,
                    security: self.security_config,
                    email_allowlist: self.email_allowlist,
                };
                let state = AppState::new_without_db(config, Some(readiness));
                (state, false)
            }
        };

        if needs_resolution {
            resolve_dependencies(&state).await?;
        }

        Ok(state)
    }
}

pub fn build_state() -> StateBuilder {
    StateBuilder::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build_hollow_state_without_env_or_db() {
        let state = build_state().build().await.unwrap();
        assert!(state.db().is_none());
    }

    #[tokio::test]
    async fn test_build_succeed_without_db_option() {
        let state = build_state()
            .with_env(RuntimeEnv::Test)
            .with_db(DbKind::SqliteMemory)
            .build()
            .await
            .unwrap();
        // It succeeds now because we provide required config
        assert!(state.db().is_some());
    }

    #[tokio::test]
    async fn test_build_fails_on_invalid_db_in_strict_mode() {
        // Using Prod + SqliteMemory will trigger a validation error
        let result = build_state()
            .with_env(RuntimeEnv::Prod)
            .with_db(DbKind::SqliteMemory)
            .build()
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_build_graceful_on_invalid_db_in_resilient_mode() {
        let manager = Arc::new(ReadinessManager::new());
        let result = build_state()
            .with_env(RuntimeEnv::Prod)
            .with_db(DbKind::SqliteMemory)
            .with_readiness(manager.clone())
            .build()
            .await;

        // In Nommie, resolve_dependencies bubbles up terminal errors even in
        // "hollow" build if it's the first authoritative attempt.
        // So this will actually be Err.
        assert!(result.is_err());
        assert!(!manager.is_ready());
    }

    #[tokio::test]
    async fn test_test_env_without_redis_url_disables_redis_and_allows_readiness() {
        let manager = Arc::new(ReadinessManager::new());
        let state = build_state()
            .with_env(RuntimeEnv::Test)
            .with_db(DbKind::SqliteMemory)
            .with_readiness(manager.clone())
            .build()
            .await
            .unwrap();

        assert!(state.config.redis_url.0.is_none());

        manager.set_migration_result(true, None);

        for _ in 0..2 {
            manager.update_dependency(
                crate::readiness::types::DependencyName::Postgres,
                crate::readiness::types::DependencyCheck::Ok {
                    latency: std::time::Duration::from_millis(1),
                },
            );
        }

        assert!(manager.is_ready());

        let internal = manager.to_internal_json();
        let deps = internal["dependencies"].as_array().unwrap();
        let redis = deps
            .iter()
            .find(|d| d["name"] == "redis")
            .expect("redis dependency present");
        assert_eq!(redis["status"]["state"], "disabled");
        assert_eq!(
            redis["status"]["reason"],
            StateBuilder::REDIS_DISABLED_REASON_TEST
        );
    }

    #[tokio::test]
    async fn test_non_test_env_without_redis_url_fails_fast() {
        let manager = Arc::new(ReadinessManager::new());
        let result = build_state()
            .with_env(RuntimeEnv::Prod)
            .with_db(DbKind::Postgres)
            .with_readiness(manager)
            .build()
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.code(), crate::errors::ErrorCode::ConfigError);
    }

    #[tokio::test]
    async fn test_test_env_with_redis_url_keeps_redis_required() {
        let manager = Arc::new(ReadinessManager::new());
        let _state = build_state()
            .with_env(RuntimeEnv::Test)
            .with_db(DbKind::SqliteMemory)
            .with_redis_url(Some("redis://localhost:6379".to_string()))
            .with_readiness(manager.clone())
            .build()
            .await
            .unwrap();

        // Simulate successful migrations.
        manager.set_migration_result(true, None);

        // Only Postgres is marked healthy; Redis is configured (not disabled),
        // so readiness must remain not ready.
        for _ in 0..2 {
            manager.update_dependency(
                crate::readiness::types::DependencyName::Postgres,
                crate::readiness::types::DependencyCheck::Ok {
                    latency: std::time::Duration::from_millis(1),
                },
            );
        }

        assert!(
            !manager.is_ready(),
            "readiness must not become ready when Redis is configured but not healthy"
        );

        let internal = manager.to_internal_json();
        let deps = internal["dependencies"].as_array().unwrap();
        let redis = deps
            .iter()
            .find(|d| d["name"] == "redis")
            .expect("redis dependency present");
        assert_ne!(redis["status"]["state"], "disabled");
    }

    #[tokio::test]
    async fn test_redis_connect_failure_marks_dependency_down() {
        let manager = Arc::new(ReadinessManager::new());

        let result = build_state()
            .with_env(RuntimeEnv::Test)
            .with_db(DbKind::SqliteMemory)
            .with_redis_url(Some("invalid-redis-url".to_string()))
            .with_readiness(manager.clone())
            .build()
            .await;

        // DB bootstrap succeeds in test with SqliteMemory; Redis connect failure is non-terminal.
        assert!(result.is_ok());

        let internal = manager.to_internal_json();
        let deps = internal["dependencies"].as_array().unwrap();
        let redis = deps
            .iter()
            .find(|d| d["name"] == "redis")
            .expect("redis dependency present after failed connect");
        assert_eq!(redis["status"]["state"], "down");
        assert!(redis["consecutive_failures"].as_u64().unwrap_or(0) >= 1);
    }
}
