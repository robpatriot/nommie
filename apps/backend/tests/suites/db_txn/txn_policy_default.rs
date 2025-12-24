//! Tests for default commit policy behavior
//!
//! This test binary runs without mod common, so it uses the OnceLock default
//! of CommitOnOk policy. It verifies that the default policy works correctly.
//!
//! Also contains tests that require committed data (e.g., database constraints)
//! since they need the CommitOnOk policy to work properly.

// Initialize logging directly (no mod common)

#[ctor::ctor]
fn init_logging() {
    backend_test_support::logging::init();
}

use backend::db::txn::with_txn;
use backend::db::txn_policy::{current, set_txn_policy, TxnPolicy};
use ulid::Ulid;

use crate::support::build_test_state;
use crate::support::db_games::{
    count_games_by_name_pool, delete_games_by_name, insert_minimal_game_for_test,
};

#[test]
fn test_policy_default_and_once_lock_behavior() {
    // These tests are in this file (not in unit tests) because this test binary
    // does not import mod common, so it uses the OnceLock default behavior.
    // Unit tests import mod common which sets policy to RollbackOnOk, making it
    // impossible to test the default CommitOnOk behavior and OnceLock mechanics.

    // Verify we start with the default policy (OnceLock is empty, so returns default)
    assert_eq!(current(), TxnPolicy::CommitOnOk);

    // Set it to CommitOnOk - this should succeed since it's the first call
    set_txn_policy(TxnPolicy::CommitOnOk);

    // The policy should now be CommitOnOk
    assert_eq!(current(), TxnPolicy::CommitOnOk);

    // Try to set it to RollbackOnOk - this should have no effect due to OnceLock
    set_txn_policy(TxnPolicy::RollbackOnOk);

    // The policy should still be CommitOnOk, proving the OnceLock behavior
    assert_eq!(current(), TxnPolicy::CommitOnOk);
}

#[actix_web::test]
async fn test_default_commit_policy_persists_then_cleans_up(
) -> Result<(), Box<dyn std::error::Error>> {
    // Verify we're using the default policy
    assert_eq!(current(), TxnPolicy::CommitOnOk);

    // Build state with a real Test DB
    let state = build_test_state().await?;

    // Use unique marker for name
    let name = Ulid::new().to_string();

    // Reads use fresh pool to check visibility post-commit
    let before = count_games_by_name_pool(&state, &name).await?;

    // Writes go through with_txn for policy parity and guardrails
    with_txn(None, &state, |txn| {
        let name = name.clone();
        Box::pin(async move {
            insert_minimal_game_for_test(txn, &name).await?;
            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    assert_eq!(count_games_by_name_pool(&state, &name).await?, before + 1);

    // Cleanup via with_txn to leave DB unchanged for other tests
    // Uses with_txn to mirror the insert pattern; commits due to CommitOnOk policy
    with_txn(None, &state, |txn| {
        let name = name.clone();
        Box::pin(async move {
            delete_games_by_name(txn, &name).await?;
            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    assert_eq!(count_games_by_name_pool(&state, &name).await?, before);

    Ok(())
}

#[actix_web::test]
async fn test_default_commit_policy_on_error() -> Result<(), Box<dyn std::error::Error>> {
    // Verify we're using the default policy
    assert_eq!(current(), TxnPolicy::CommitOnOk);

    // Build state with a real Test DB
    let state = build_test_state().await?;

    // Use unique marker for name
    let name = Ulid::new().to_string();
    let before = count_games_by_name_pool(&state, &name).await?;

    // Insert then return error; should rollback (no commit)
    let result = with_txn(None, &state, |txn| {
        let name = name.clone();
        Box::pin(async move {
            insert_minimal_game_for_test(txn, &name).await?;
            Err::<(), _>(backend::AppError::internal(
                backend::ErrorCode::InternalError,
                "test error triggered",
                std::io::Error::other("test error for rollback verification"),
            ))
        })
    })
    .await;

    assert!(result.is_err());
    assert_eq!(count_games_by_name_pool(&state, &name).await?, before);

    Ok(())
}

// Join code uniqueness tests have been removed along with join code support.
