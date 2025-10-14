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

    // Test DB connectivity with an actual database operation
    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Perform a simple query to verify the database connection works
            use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

            let stmt = Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT 1 as test_value".to_owned(),
            );

            let result = txn.execute(stmt).await?;
            assert_eq!(result.rows_affected(), 1);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
