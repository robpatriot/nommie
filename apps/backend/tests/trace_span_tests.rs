//! Trace span integration tests
//!
//! This test binary does NOT import mod common because these tests need to
//! set their own global tracing subscriber for testing purposes.
//!
//! Run these tests:
//!   cargo test --test trace_span_tests

// NOTE: Do NOT add `mod common;` here - these tests set their own global subscriber

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

#[path = "suites/routes/trace_span.rs"]
mod trace_span;
