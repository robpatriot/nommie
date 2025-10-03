#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::todo, clippy::unimplemented))]

pub mod adapters;
pub mod auth;
pub mod config;
pub mod db;
pub mod domain;
pub mod entities;
pub mod error;
pub mod errors;
pub mod extractors;
pub mod infra;
pub mod logging;
pub mod middleware;
pub mod repos;
pub mod routes;
pub mod services;
pub mod state;
pub mod utils;
pub mod web;

// Re-exports for public API
pub use auth::jwt::{mint_access_token, verify_access_token, Claims};
pub use config::db::{db_url, DbOwner, DbProfile};
pub use db::txn::{with_txn, SharedTxn};
pub use db::txn_policy::{set_txn_policy, TxnPolicy};
pub use error::AppError;
pub use errors::ErrorCode;
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
// Auto-initialize logging for unit tests
#[cfg(test)]
#[ctor::ctor]
fn init_test_logging() {
    backend_test_support::logging::init();
}
