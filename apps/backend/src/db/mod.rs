pub mod shared_pool_cache;
pub mod txn;
pub mod txn_policy;

use std::time::Duration;

use sea_orm::DatabaseConnection;

use crate::error::AppError;
use crate::readiness::types::{DependencyCheck, DependencyName};
use crate::state::app_state::AppState;

/// Centralized helper to access the database connection from AppState.
///
/// This is the canonical way to access the database from application code.
/// It returns a borrowed reference to the DatabaseConnection if available,
/// or an AppError::db_unavailable() if the database is not configured.
/// When returning an error, also reports Postgres as down to the readiness
/// manager so /readyz can transition to not ready.
pub fn require_db(state: &AppState) -> Result<DatabaseConnection, AppError> {
    match state.db() {
        Some(db) => Ok(db),
        None => {
            let err = AppError::db_unavailable(
                "database unavailable",
                crate::error::Sentinel("database not configured in AppState"),
                Some(1),
            );
            state.readiness().update_dependency(
                DependencyName::Postgres,
                DependencyCheck::Down {
                    error: err.to_string(),
                    latency: Duration::ZERO,
                },
            );
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actix_web::http::StatusCode;
    use actix_web::ResponseError;
    use backend_test_support::problem_details::assert_problem_details_from_http_response;

    use super::*;
    use crate::auth::google::{MockGoogleVerifier, VerifiedGoogleClaims};
    use crate::config::db::RuntimeEnv;
    use crate::state::admission_mode::AdmissionMode;
    use crate::state::app_state::{AppConfig, Secret};
    use crate::state::security_config::SecurityConfig;

    fn test_google_verifier() -> Arc<dyn crate::auth::google::GoogleIdTokenVerifier> {
        Arc::new(MockGoogleVerifier::new(VerifiedGoogleClaims {
            sub: "test-sub".to_string(),
            email: "test@example.com".to_string(),
            name: None,
        }))
    }

    #[tokio::test]
    async fn test_require_db_without_db() {
        let config = AppConfig {
            env: RuntimeEnv::Test,
            db_url: Secret("".to_string()),
            redis_url: Secret(None),
            security: SecurityConfig::default(),
            google_verifier: test_google_verifier(),
            admission_mode: AdmissionMode::Open,
        };
        let app_state = AppState::new_without_db(config, None);

        let result = require_db(&app_state);
        assert!(result.is_err());

        if let Err(error) = result {
            // Use contract assertions via HTTP path
            let response = error.error_response();
            assert_problem_details_from_http_response(
                response,
                "SERVICE_UNAVAILABLE",
                StatusCode::SERVICE_UNAVAILABLE,
                Some("Service temporarily unavailable"),
            )
            .await;
        } else {
            panic!("Expected DbUnavailable error");
        }
    }

    #[tokio::test]
    async fn test_require_db_error_code() {
        let config = AppConfig {
            env: RuntimeEnv::Test,
            db_url: Secret("".to_string()),
            redis_url: Secret(None),
            security: SecurityConfig::default(),
            google_verifier: test_google_verifier(),
            admission_mode: AdmissionMode::Open,
        };
        let app_state = AppState::new_without_db(config, None);

        let result = require_db(&app_state);
        assert!(result.is_err());

        if let Err(error) = result {
            // Use contract assertions via HTTP path
            let response = error.error_response();
            assert_problem_details_from_http_response(
                response,
                "SERVICE_UNAVAILABLE",
                StatusCode::SERVICE_UNAVAILABLE,
                Some("Service temporarily unavailable"),
            )
            .await;
        } else {
            panic!("Expected error");
        }
    }
}
