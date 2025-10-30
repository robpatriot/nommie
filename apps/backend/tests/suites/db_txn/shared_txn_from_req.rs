// Tests for SharedTxn::from_req helper method
//
// These tests verify that the from_req method correctly extracts SharedTxn
// from request extensions and returns None when not present.

use actix_web::test;
use backend::config::db::{DbKind, RuntimeEnv};
use backend::db::require_db;
use backend::infra::state::build_state;
use backend::SharedTxn;

#[actix_web::test]
async fn test_from_req_injected_case() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with Test DB
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;

    // Get pooled DB and open a shared txn
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Build an HttpRequest and inject the shared txn
    let mut req = test::TestRequest::default().to_http_request();
    shared.inject(&mut req);

    // Test that from_req returns Some
    let extracted = SharedTxn::from_req(&req);
    assert!(extracted.is_some());

    // Verify it is the same underlying Arc target using Arc::ptr_eq
    let extracted = extracted.unwrap();
    assert!(std::sync::Arc::ptr_eq(&shared.0, &extracted.0));

    // Clean up - drop extracted reference first, then request, then rollback
    drop(extracted);
    drop(req);
    shared.rollback().await.unwrap();

    Ok(())
}

#[actix_web::test]
async fn test_from_req_missing_case() -> Result<(), Box<dyn std::error::Error>> {
    // Build a fresh HttpRequest with no injection
    let req = test::TestRequest::default().to_http_request();

    // Test that from_req returns None
    let extracted = SharedTxn::from_req(&req);
    assert!(extracted.is_none());

    Ok(())
}
