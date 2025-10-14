//! Commit policy tests
//!
//! This test binary intentionally does NOT import mod common, so it uses the
//! OnceLock default of CommitOnOk policy. It verifies that the default policy
//! works correctly.
//!
//! Run these tests:
//!   cargo test --test commit_policy_tests

// NOTE: Do NOT add `mod common;` here - these tests need default CommitOnOk policy

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

#[path = "suites/db_txn/txn_policy_default.rs"]
mod txn_policy_default;
