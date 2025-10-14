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
mod support;

#[path = "suites/regression"]
mod regression {
    pub mod game_flow_ai;
}
