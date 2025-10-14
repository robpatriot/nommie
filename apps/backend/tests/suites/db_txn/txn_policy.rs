//! Integration tests for transaction policy functionality.
//!
//! These tests verify that the TxnPolicy OnceLock works correctly in isolation
//! within each integration test binary.

use backend::db::txn_policy::{current, set_txn_policy, TxnPolicy};

#[test]
fn test_set_policy_to_rollback_on_ok() {
    // Set policy to RollbackOnOk
    set_txn_policy(TxnPolicy::RollbackOnOk);
    assert_eq!(current(), TxnPolicy::RollbackOnOk);
}

#[test]
fn test_set_policy_is_idempotent() {
    // Set policy to RollbackOnOk first
    set_txn_policy(TxnPolicy::RollbackOnOk);
    assert_eq!(current(), TxnPolicy::RollbackOnOk);

    // Attempting to set it again should have no effect
    set_txn_policy(TxnPolicy::CommitOnOk);
    assert_eq!(current(), TxnPolicy::RollbackOnOk);
}
