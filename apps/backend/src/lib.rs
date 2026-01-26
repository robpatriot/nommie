//! Nommie Backend Library
//!
//! This crate provides both a library API for integration testing and a binary entry point.
//! The library exposes all modules needed for testing and programmatic use.

#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented
    )
)]

// Public modules - these are accessible to both the binary and integration tests
pub mod adapters;
pub mod ai;
pub mod auth;
pub mod bin_support;
pub mod config;
pub mod db;
pub mod domain;
pub mod entities;
pub mod error;
pub mod errors;
pub mod extractors;
pub mod http;
pub mod infra;
pub mod logging;
pub mod middleware;
pub mod protocol;
pub mod repos;
pub mod routes;
pub mod services;
pub mod state;
pub mod telemetry;
pub mod trace_ctx;
pub mod utils;
pub mod ws;

// Re-export commonly used types for easier imports
pub use error::AppError;
pub use errors::ErrorCode;
pub use extractors::current_user::CurrentUser;
pub use extractors::game_id::GameId;
pub use extractors::game_membership::GameMembership;
pub use extractors::ValidatedJson;

// Prelude module for convenient imports
// Usage: `use backend::prelude::*;`
pub mod prelude {
    // Error types
    // Database utilities
    pub use super::db::require_db;
    pub use super::db::txn::{with_txn, SharedTxn};
    pub use super::error::AppError;
    pub use super::errors::ErrorCode;
    // Extractors
    pub use super::extractors::current_user::CurrentUser;
    pub use super::extractors::game_id::GameId;
    pub use super::extractors::game_membership::GameMembership;
    pub use super::extractors::ValidatedJson;
    // State building
    pub use super::infra::state::build_state;
    pub use super::state::security_config::SecurityConfig;
}

// Auto-initialize logging for unit tests
#[cfg(test)]
#[ctor::ctor]
fn init_test_logging() {
    backend_test_support::logging::init();
}
