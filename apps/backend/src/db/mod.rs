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
    state.db().ok_or_else(AppError::db_unavailable)
}

#[cfg(test)]
mod tests {
    use actix_web::ResponseError;

    use super::*;
    use crate::state::security_config::SecurityConfig;

    #[test]
    fn test_require_db_without_db() {
        let security_config = SecurityConfig::default();
        let app_state = AppState::new_without_db(security_config);

        let result = require_db(&app_state);
        assert!(result.is_err());

        if let Err(AppError::DbUnavailable) = result {
            // Expected error type
        } else {
            panic!("Expected DbUnavailable error");
        }
    }

    #[test]
    fn test_require_db_error_code() {
        let security_config = SecurityConfig::default();
        let app_state = AppState::new_without_db(security_config);

        let result = require_db(&app_state);
        assert!(result.is_err());

        if let Err(error) = result {
            // Test the error response to verify the problem details
            let response = error.error_response();
            assert_eq!(
                response.status(),
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            );

            // We can't easily test the JSON body in unit tests without deserializing,
            // but we can verify the error type and status
            assert!(matches!(error, AppError::DbUnavailable));
        } else {
            panic!("Expected error");
        }
    }
}
