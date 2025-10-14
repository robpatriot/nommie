//! HTTP routes and middleware tests
//!
//! Tests for HTTP handlers, extractors, validation, error formatting,
//! healthcheck, and state management.
//!
//! Run all routes tests:
//!   cargo test --test routes_tests
//!
//! Run specific routes tests:
//!   cargo test --test routes_tests routes::handler_players::

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

#[path = "suites/routes"]
mod routes {
    pub mod error_codes_unique;
    pub mod error_mappings;
    pub mod error_shape;
    pub mod extractor_current_user_db;
    pub mod extractor_game_id;
    pub mod extractor_game_membership;
    pub mod extractor_game_membership_roles;
    pub mod handler_players;
    pub mod healthcheck;
    pub mod state_builder;
    // trace_span is in its own test binary (trace_span_tests.rs)
    pub mod validated_json;
}
