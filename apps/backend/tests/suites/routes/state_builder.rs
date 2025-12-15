use backend::config::db::RuntimeEnv;
use backend::db::txn::with_txn;
use backend::AppError;
use backend::infra::state::build_state;

use crate::support::resolve_test_db_kind;

#[tokio::test]
async fn builds_without_db() -> Result<(), AppError> {
    // This should succeed and create an AppState without a database
    let state = build_state().build().await?;
    assert!(state.db().is_none());
    Ok(())
}

#[tokio::test]
async fn builds_with_test_db() -> Result<(), AppError> {
    let db_kind = resolve_test_db_kind()?;
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(db_kind)
        .build()
        .await?;
    assert!(state.db().is_some());

    // Test DB connectivity with an actual database operation
    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Perform a simple query to verify the database connection works
            use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

            let backend = DatabaseBackend::from(db_kind);

            let stmt = Statement::from_string(backend, "SELECT 1 as test_value".to_owned());

            let row = txn.query_one(stmt).await?.expect("should get a row");
            let value: i32 = row.try_get("", "test_value")?;
            assert_eq!(value, 1);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
