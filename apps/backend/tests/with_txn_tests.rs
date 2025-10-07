//! Tests for rollback policy behavior
//!
//! This module now uses the common initialization which sets the
//! RollbackOnOk policy and does not persist writes to the database.
mod common;
mod support;

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::db::txn_policy::{current, TxnPolicy};
use backend::infra::state::build_state;
use support::db_games::{count_games_by_name_pool, insert_game_stub};
use tracing::debug;
use ulid::Ulid;

#[actix_web::test]
async fn test_rollback_policy() -> Result<(), Box<dyn std::error::Error>> {
    // Verify we're using the rollback policy
    assert_eq!(current(), TxnPolicy::RollbackOnOk);

    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Use unique marker for name
    let name = Ulid::new().to_string();

    // Reads use fresh pool to check visibility post-rollback
    let before = count_games_by_name_pool(&state, &name).await?;

    // Writes go through with_txn for policy parity and guardrails
    with_txn(None, &state, |txn| {
        let name = name.clone();
        Box::pin(async move {
            insert_game_stub(txn, &name).await?;
            debug!("inserted games row inside txn");
            Ok::<_, backend::error::AppError>(())
        })
    })
    .await?;

    // Outside the transaction, verify the row is not present (rolled back)
    let after = count_games_by_name_pool(&state, &name).await?;
    debug!("query after txn returned count: {}", after);
    assert_eq!(after, before, "row should not persist after rollback-on-ok");

    Ok(())
}

#[actix_web::test]
async fn test_rollback_policy_on_error() -> Result<(), Box<dyn std::error::Error>> {
    // Verify we're using the rollback policy
    assert_eq!(current(), TxnPolicy::RollbackOnOk);

    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Use unique marker for name
    let name = Ulid::new().to_string();
    let before = count_games_by_name_pool(&state, &name).await?;

    // Insert inside the transaction, then return an error to force rollback
    let result = with_txn(None, &state, |txn| {
        let name = name.clone();
        Box::pin(async move {
            insert_game_stub(txn, &name).await?;
            debug!("inserted games row inside txn before error");

            Err::<(), _>(backend::error::AppError::Internal {
                code: backend::errors::ErrorCode::InternalError,
                detail: "test error".to_string(),
            })
        })
    })
    .await;

    // Verify the operation failed
    assert!(result.is_err());

    // Outside the transaction, verify no row with the unique name exists
    let after = count_games_by_name_pool(&state, &name).await?;
    debug!("query after txn (error case) returned count: {}", after);
    assert_eq!(
        after, before,
        "row should not persist after rollback on error"
    );

    Ok(())
}
