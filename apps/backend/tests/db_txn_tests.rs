//! Database transaction tests
//!
//! Tests for transaction handling, nesting, rollback/commit policies,
//! and shared transaction support.
//!
//! Run all transaction tests:
//!   cargo test --test db_txn_tests
//!
//! Run specific transaction tests:
//!   cargo test --test db_txn_tests db_txn::with_txn::

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

#[path = "suites/db_txn"]
mod db_txn {
    pub mod shared_txn;
    pub mod shared_txn_from_req;
    pub mod txn_policy;
    // txn_policy_default is in its own test binary (commit_policy_tests.rs)
    pub mod with_txn;
    pub mod with_txn_shared_txn;
}
