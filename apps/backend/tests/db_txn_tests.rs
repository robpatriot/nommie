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
mod support;

#[path = "suites/db_txn"]
mod db_txn {
    pub mod shared_txn;
    pub mod shared_txn_from_req;
    pub mod txn_policy;
    // txn_policy_default is in its own test binary (commit_policy_tests.rs)
    pub mod with_txn;
    pub mod with_txn_shared_txn;
}
