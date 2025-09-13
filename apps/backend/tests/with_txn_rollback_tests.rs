//! Tests for rollback policy behavior (includes test_init.rs)
//!
//! This module includes the test_init.rs module, so it should use the
//! RollbackOnOk policy and not persist writes to the database.
#[path = "support/test_init.rs"]
mod test_init;

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::db::txn_policy::{current, TxnPolicy};
use backend::infra::state::build_state;

#[actix_web::test]
async fn test_rollback_policy() -> Result<(), Box<dyn std::error::Error>> {
    // Verify we're using the rollback policy
    assert_eq!(current(), TxnPolicy::RollbackOnOk);

    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Test that with_txn works with rollback policy
    let result = with_txn(None, &state, |_txn| async {
        Ok::<_, backend::error::AppError>("success")
    })
    .await?;

    // Verify the operation succeeded
    assert_eq!(result, "success");

    Ok(())
}

#[actix_web::test]
async fn test_rollback_policy_on_error() -> Result<(), Box<dyn std::error::Error>> {
    // Verify we're using the rollback policy
    assert_eq!(current(), TxnPolicy::RollbackOnOk);

    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Test that with_txn handles errors correctly with rollback policy
    let result = with_txn(None, &state, |_txn| async {
        Err::<String, _>(backend::error::AppError::Internal {
            detail: "test error".to_string(),
            trace_id: None,
        })
    })
    .await;

    // Verify the operation failed
    assert!(result.is_err());

    Ok(())
}
