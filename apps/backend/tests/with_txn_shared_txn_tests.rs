//! Tests for SharedTxn reuse behavior
//!
//! These tests verify that SharedTxn bypasses the transaction policy
//! and that with_txn does not perform commit/rollback operations.
#[path = "support/shared_txn.rs"]
mod shared_txn;

use actix_web::test;
use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::infra::state::build_state;

#[actix_web::test]
async fn test_shared_txn_reuse_bypasses_policy() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Open a shared transaction
    let shared = shared_txn::open(&state.db).await;

    // Create a mutable request and inject the shared transaction
    let mut req = test::TestRequest::default().to_http_request();
    shared_txn::inject(&mut req, &shared);

    // Write via with_txn using the shared transaction
    let result = with_txn(Some(&req), &state, |_txn| async {
        Ok::<_, backend::error::AppError>("success")
    })
    .await?;

    // Verify the operation succeeded
    assert_eq!(result, "success");

    // Drop the request to release the shared transaction reference
    drop(req);

    // Roll back the shared transaction explicitly to prove tests own rollback
    shared_txn::rollback(shared).await.unwrap();

    Ok(())
}

#[actix_web::test]
async fn test_shared_txn_reuse_commit_behavior() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Open a shared transaction
    let shared = shared_txn::open(&state.db).await;

    // Create a mutable request and inject the shared transaction
    let mut req = test::TestRequest::default().to_http_request();
    shared_txn::inject(&mut req, &shared);

    // Write via with_txn using the shared transaction
    let result = with_txn(Some(&req), &state, |_txn| async {
        Ok::<_, backend::error::AppError>("success")
    })
    .await?;

    // Verify the operation succeeded
    assert_eq!(result, "success");

    // Drop the request to release the shared transaction reference
    drop(req);

    // Commit the shared transaction explicitly
    shared_txn::commit(shared).await.unwrap();

    Ok(())
}
