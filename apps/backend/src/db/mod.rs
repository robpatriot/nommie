pub mod shared_pool_cache;
pub mod txn;
pub mod txn_policy;

use sea_orm::DatabaseConnection;

use crate::error::AppError;
use crate::state::app_state::AppState;

/// Centralized helper to access the database connection from AppState.
///
/// This is the canonical way to access the database from application code.
/// It returns a borrowed reference to the DatabaseConnection if available,
/// or an AppError::db_unavailable() if the database is not configured.
pub fn require_db(state: &AppState) -> Result<&DatabaseConnection, AppError> {
    state.db().ok_or_else(|| {
        AppError::db_unavailable(
            "database unavailable",
            crate::error::Sentinel("database not configured in AppState"),
            Some(1),
        )
    })
}

#[cfg(test)]
mod tests {
    use actix_web::http::StatusCode;
    use actix_web::ResponseError;
    use backend_test_support::problem_details::assert_problem_details_from_http_response;

    use super::*;
    use crate::state::security_config::SecurityConfig;

    #[tokio::test]
    async fn test_require_db_without_db() {
        let security_config = SecurityConfig::default();
        let app_state = AppState::new_without_db(security_config);

        let result = require_db(&app_state);
        assert!(result.is_err());

        if let Err(error) = result {
            // Use contract assertions via HTTP path
            let response = error.error_response();
            assert_problem_details_from_http_response(
                response,
                "DB_UNAVAILABLE",
                StatusCode::SERVICE_UNAVAILABLE,
                Some("database unavailable"),
            )
            .await;
        } else {
            panic!("Expected DbUnavailable error");
        }
    }

    #[tokio::test]
    async fn test_require_db_error_code() {
        let security_config = SecurityConfig::default();
        let app_state = AppState::new_without_db(security_config);

        let result = require_db(&app_state);
        assert!(result.is_err());

        if let Err(error) = result {
            // Use contract assertions via HTTP path
            let response = error.error_response();
            assert_problem_details_from_http_response(
                response,
                "DB_UNAVAILABLE",
                StatusCode::SERVICE_UNAVAILABLE,
                Some("database unavailable"),
            )
            .await;
        } else {
            panic!("Expected error");
        }
    }
}
