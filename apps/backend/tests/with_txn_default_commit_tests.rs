//! Tests for default commit behavior (no test_init.rs included)
//!
//! These tests run without the test_init.rs module, so they should use the default
//! CommitOnOk policy and persist writes to the database.
use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::db::txn_policy::{current, TxnPolicy};
use backend::infra::state::build_state;

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
