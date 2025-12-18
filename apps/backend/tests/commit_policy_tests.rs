// Commit policy tests
//
// This test binary intentionally does NOT import mod common, so it uses the
// OnceLock default of CommitOnOk policy. It verifies that the default policy
// works correctly.
//
// Run these tests:
//   cargo test --test commit_policy_tests

// NOTE: Do NOT add `mod common;` here - these tests need default CommitOnOk policy

mod support;

#[path = "suites/db_txn/txn_policy_default.rs"]
mod txn_policy_default;

#[path = "suites/services/ensure_user_unique_retry_commit.rs"]
mod ensure_user_unique_retry_commit;
