pub mod txn;
pub mod txn_policy;

use std::any::Any;
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction};

use crate::error::AppError;
use crate::state::app_state::AppState;

/// Database connection abstraction that wraps SeaORM's ConnectionTrait.
/// 
/// This trait provides a clean boundary between the domain layer and SeaORM.
/// Services and repositories use this trait instead of directly importing SeaORM.
pub trait DbConn: ConnectionTrait + Send + Sync + Any {}

// Implement DbConn for SeaORM types
impl DbConn for DatabaseConnection {}
impl DbConn for DatabaseTransaction {}

// DbConn is defined in this module; consumers should `use crate::db::DbConn;`

/// Centralized helper to access the database connection from AppState.
///
/// This is the canonical way to access the database from application code.
/// It returns a borrowed reference to the DatabaseConnection if available,
/// or an AppError::db_unavailable() if the database is not configured.
pub fn require_db(state: &AppState) -> Result<&DatabaseConnection, AppError> {
    state.db().ok_or_else(AppError::db_unavailable)
}

/// Downcast a `&dyn DbConn` to a `&DatabaseConnection` if possible
pub fn as_database_connection(conn: &dyn DbConn) -> Option<&DatabaseConnection> {
    let any = conn as &dyn Any;
    any.downcast_ref::<DatabaseConnection>()
}

/// Downcast a `&dyn DbConn` to a `&DatabaseTransaction` if possible
pub fn as_database_transaction(conn: &dyn DbConn) -> Option<&DatabaseTransaction> {
    let any = conn as &dyn Any;
    any.downcast_ref::<DatabaseTransaction>()
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
                Some("Database unavailable"),
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
                Some("Database unavailable"),
            )
            .await;
        } else {
            panic!("Expected error");
        }
    }
}
