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

#[path = "suites/db_txn/mod.rs"]
mod db_txn;
