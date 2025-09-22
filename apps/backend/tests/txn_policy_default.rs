//! Tests for default commit policy behavior
//!
//! This test binary runs without mod common, so it uses the OnceLock default
//! of CommitOnOk policy. It verifies that the default policy works correctly.

// Initialize logging directly (no mod common)
#[ctor::ctor]
fn init_logging() {
    backend_test_support::logging::init();
}

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::db::txn_policy::{current, TxnPolicy};
use backend::infra::state::build_state;

#[test]
fn test_policy_is_commit_on_ok() {
    // This test binary does not import mod common, so it should see the
    // OnceLock default of CommitOnOk policy
    assert_eq!(current(), TxnPolicy::CommitOnOk);
}

#[actix_web::test]
async fn test_default_commit_policy() -> Result<(), Box<dyn std::error::Error>> {
    // Verify we're using the default policy
    assert_eq!(current(), TxnPolicy::CommitOnOk);

    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Test that with_txn works with default commit policy
    let result = with_txn(None, &state, |_txn| {
        Box::pin(async { Ok::<_, backend::error::AppError>("success") })
    })
    .await?;

    // Verify the operation succeeded
    assert_eq!(result, "success");

    Ok(())
}

#[actix_web::test]
async fn test_default_commit_policy_on_error() -> Result<(), Box<dyn std::error::Error>> {
    // Verify we're using the default policy
    assert_eq!(current(), TxnPolicy::CommitOnOk);

    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Test that with_txn handles errors correctly with default commit policy
    let result = with_txn(None, &state, |_txn| {
        Box::pin(async {
            Err::<String, _>(backend::error::AppError::Internal {
                detail: "test error".to_string(),
            })
        })
    })
    .await;

    // Verify the operation failed
    assert!(result.is_err());

    Ok(())
}
