use std::sync::Arc;

use crate::auth::google::{MockGoogleVerifier, VerifiedGoogleClaims};
use crate::config::db::{DbKind, RuntimeEnv};
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
    redis_url: Option<String>,
    readiness: Option<Arc<ReadinessManager>>,
    google_verifier: Option<std::sync::Arc<dyn crate::auth::google::GoogleIdTokenVerifier>>,
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
            let (needs_resolution, healthy_check) = match realtime {
                None => (true, None),
                Some(broker) => {
                    let subscriber_alive = broker.is_subscriber_alive();
                    if !subscriber_alive {
                        tracing::info!(
                            "readiness: Redis subscriber dead, triggering broker replacement"
                        );
                    }
                    let publisher_check = broker.check_publisher().await;
                    if !publisher_check.is_ok() {
                        tracing::info!(
                            "readiness: Redis publisher unhealthy, triggering broker replacement"
                        );
                    }
                    let needs = !subscriber_alive || !publisher_check.is_ok();
                    (needs, Some(publisher_check))
                }
            };

            if needs_resolution {
                let start = std::time::Instant::now();
                match RealtimeBroker::connect(url, state.readiness().clone()).await {
                    Ok(broker) => {
                        state.set_realtime(broker);
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
                    }
                }
            } else if let Some(check) = healthy_check {
                // Broker exists, subscriber alive, publisher ok – update readiness.
                if check.is_ok() {
                    state
                        .readiness()
                        .update_dependency(crate::readiness::types::DependencyName::Redis, check);
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

    pub fn with_redis_url(mut self, redis_url: Option<String>) -> Self {
        self.redis_url = redis_url;
        self
    }

    pub fn with_google_verifier(
        mut self,
        verifier: std::sync::Arc<dyn crate::auth::google::GoogleIdTokenVerifier>,
    ) -> Self {
        self.google_verifier = Some(verifier);
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

        let google_verifier = self.google_verifier.unwrap_or_else(|| {
            Arc::new(MockGoogleVerifier::new(VerifiedGoogleClaims {
                sub: "test-google-sub".to_string(),
                email: "test@example.com".to_string(),
                name: Some("Test User".to_string()),
            }))
        });

        let (state, needs_resolution) = match (self.env, self.db_kind) {
            (Some(env), Some(db_kind)) => {
                let config = AppConfig {
                    env,
                    db_kind,
                    db_url,
                    redis_url,
                    security: self.security_config,
                    google_verifier,
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
                    google_verifier,
                };
                let state = AppState::new_without_db(config, Some(readiness));
                (state, false)
            }
        };

        if needs_resolution {
            resolve_dependencies(&state).await?;
            state.readiness().mark_initial_resolution_success();
        }

        Ok(state)
    }
}

pub fn build_state() -> StateBuilder {
    StateBuilder::default()
}
