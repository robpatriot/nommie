mod common;
mod support;

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::infra::state::build_state;

#[tokio::test]
async fn builds_without_db() {
    // This should succeed and create an AppState without a database
    let state = build_state().build().await.unwrap();
    assert!(state.db().is_none());
}

#[tokio::test]
async fn builds_with_test_db() -> Result<(), AppError> {
    let state = build_state().with_db(DbProfile::Test).build().await?;
    assert!(state.db().is_some());

    // Test DB connectivity with a no-op transaction
    with_txn(None, &state, |_txn| {
        Box::pin(async { Ok::<_, AppError>(()) })
    })
    .await?;

    Ok(())
}
