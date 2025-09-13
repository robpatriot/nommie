use std::sync::OnceLock;

/// Transaction policy that determines whether transactions should be committed or rolled back on success
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxnPolicy {
    /// Commit the transaction when the operation succeeds (default behavior)
    CommitOnOk,
    /// Rollback the transaction when the operation succeeds (for testing)
    RollbackOnOk,
}

static POLICY: OnceLock<TxnPolicy> = OnceLock::new();

/// Get the current transaction policy.
///
/// Returns `CommitOnOk` if no policy has been set (default behavior).
pub fn current() -> TxnPolicy {
    POLICY.get().copied().unwrap_or(TxnPolicy::CommitOnOk)
}

/// Set the transaction policy for the process.
///
/// This function is idempotent - only the first call will have any effect.
/// Subsequent calls will be ignored.
pub fn set_txn_policy(policy: TxnPolicy) {
    let _ = POLICY.set(policy);
}
