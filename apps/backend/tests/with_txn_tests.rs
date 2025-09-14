#[path = "support/mock_strict.rs"]
mod mock_strict;
#[path = "support/shared_txn.rs"]
mod shared_txn;

use std::panic::AssertUnwindSafe;

use actix_web::test;
use backend::config::db::DbProfile;
use backend::db::txn::{with_txn, ERR_MOCK_STRICT_NO_SHARED_TXN};
use backend::infra::state::build_state;
use futures_util::FutureExt; // for .catch_unwind()

#[actix_web::test]
async fn test_blocks_on_mock_strict_without_shared_txn() {
    // Build state with a real Test DB
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .unwrap();

    // Mark the connection as mock-strict
    mock_strict::register_mock_strict_connection(&state.db);
    assert!(mock_strict::is_registered_mock_strict(&state.db));
    assert!(backend::infra::mock_strict::is_mock_strict(&state.db));

    // Create an HttpRequest without injecting a shared txn
    let req = test::TestRequest::default().to_http_request();

    // Execute the code path that should panic, and assert the panic payload matches our constant
    let result = AssertUnwindSafe(async {
        let _ = with_txn(Some(&req), &state, |_txn| Box::pin(async { Ok(()) })).await;
    })
    .catch_unwind()
    .await;

    match result {
        Ok(_) => panic!("expected panic, but future completed successfully"),
        Err(payload) => {
            // Try &str first, then String
            let msg = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(|s| s.as_str()))
                .unwrap_or("<unknown panic payload>");
            assert_eq!(msg, ERR_MOCK_STRICT_NO_SHARED_TXN);
        }
    }
}

#[actix_web::test]
async fn test_allows_with_shared_txn_on_mock_strict_no_auto_commit(
) -> Result<(), Box<dyn std::error::Error>> {
    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Mark the connection as mock-strict
    mock_strict::register_mock_strict_connection(&state.db);

    // Open a shared txn
    let shared = shared_txn::open(&state.db).await;

    // Create a mutable request and inject the shared txn
    let mut req = test::TestRequest::default().to_http_request();
    shared_txn::inject(&mut req, &shared);

    // Call with_txn and assert it returns Ok(123)
    let result = with_txn(Some(&req), &state, |_txn| {
        Box::pin(async { Ok::<_, backend::error::AppError>(123) })
    })
    .await?;

    assert_eq!(result, 123);

    // Drop the request to release the shared transaction reference
    drop(req);

    // Roll back explicitly to prove tests own rollback
    shared_txn::rollback(shared).await.unwrap();

    Ok(())
}

#[actix_web::test]
async fn test_allows_auto_commit_on_real_db_without_shared_txn(
) -> Result<(), Box<dyn std::error::Error>> {
    // Build state with a real Test DB (do not register mock-strict)
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Call with_txn with None for request and assert it returns Ok("ok")
    let result = with_txn(None, &state, |_txn| {
        Box::pin(async { Ok::<_, backend::error::AppError>("ok") })
    })
    .await?;

    assert_eq!(result, "ok");

    Ok(())
}
