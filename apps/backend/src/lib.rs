#![deny(clippy::wildcard_imports)]
#![cfg_attr(test, allow(clippy::wildcard_imports))]

pub mod auth;
pub mod config;
pub mod entities;
pub mod error;
pub mod extractors;
pub mod health;
pub mod infra;
pub mod middleware;
pub mod routes;
pub mod services;
pub mod state;

#[cfg(test)]
pub mod test_bootstrap;

// Re-exports for public API
pub use auth::jwt::{mint_access_token, verify_access_token, Claims};
pub use config::db::{db_url, DbOwner, DbProfile};
pub use error::AppError;
pub use extractors::auth_token::AuthToken;
pub use extractors::current_user::{BackendClaims, CurrentUser};
pub use extractors::current_user_db::CurrentUserRecord;
pub use extractors::game_id::GameId;
pub use extractors::jwt::JwtClaims;
pub use infra::db::connect_db;
pub use middleware::cors::cors_middleware;
pub use middleware::request_trace::RequestTrace;
pub use middleware::structured_logger::StructuredLogger;
pub use middleware::trace_span::TraceSpan;
pub use state::app_state::AppState;
pub use state::security_config::SecurityConfig;

// Prelude for test convenience
pub mod prelude {
    pub use super::auth::jwt::*;
    pub use super::config::db::*;
    pub use super::error::*;
    pub use super::extractors::*;
    pub use super::infra::*;
    pub use super::middleware::*;
    pub use super::state::*;
}

// Auto-initialize logging for unit tests
#[cfg(test)]
#[ctor::ctor]
fn init_test_logging() {
    test_bootstrap::logging::init();
}
