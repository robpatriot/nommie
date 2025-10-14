//! Regression and slow tests
//!
//! Tests that are slow or require special feature flags.
//! These are typically excluded from normal test runs.
//!
//! Run all regression tests (when feature enabled):
//!   cargo test --test regression_tests --features slow-tests
//!
//! Run with nextest filter:
//!   cargo nextest run -E 'test(~"regression::")' --features slow-tests

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

#[path = "suites/regression"]
mod regression {
    pub mod game_flow_ai;
}
