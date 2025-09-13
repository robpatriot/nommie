#[path = "support/mock_strict.rs"]
mod mock_strict;
#[path = "support/shared_txn.rs"]
mod shared_txn;

use actix_web::test;
use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::infra::state::build_state;

#[actix_web::test]
#[should_panic(
    expected = "MockStrict DB blocked a query because no shared test transaction was provided. Either use .with_db(DbProfile::Test) for a real test DB, or inject a shared transaction into the request extensions (see tests/support/shared_txn.rs"
)]
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

    // Call with_txn and assert it panics with the exact message
    let _ = with_txn(Some(&req), &state, |_txn| async { Ok(()) }).await;
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
    let result = with_txn(Some(&req), &state, |_txn| async {
        Ok::<_, backend::error::AppError>(123)
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
    let result = with_txn(None, &state, |_txn| async {
        Ok::<_, backend::error::AppError>("ok")
    })
    .await?;

    assert_eq!(result, "ok");

    Ok(())
}
