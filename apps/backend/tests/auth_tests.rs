//! Authentication and authorization tests
//!
//! Tests for authentication flows, JWT validation, and auth extractors.
//!
//! Run all auth tests:
//!   cargo test --test auth_tests
//!
//! Run specific auth tests:
//!   cargo test --test auth_tests auth::login::

mod common;

#[path = "support"]
#[allow(dead_code)]
mod support {
    pub mod app_builder;
    pub mod auth;
    pub mod db_games;
    pub mod db_memberships;
    pub mod domain_gens;
    pub mod domain_prop_helpers;
    pub mod factory;
    pub mod game_phases;
    pub mod game_setup;
    pub mod games_sea_helpers;
    pub mod snapshot_helpers;
    pub mod test_utils;
    pub mod trick_helpers;
}

#[path = "suites/auth"]
mod auth {
    pub mod extractor_auth;
    pub mod login;
}
