mod support;

use backend::config::db::DbProfile;
use backend::infra::state::build_state;

#[tokio::test]
async fn builds_without_db() {
    // This should succeed and create an AppState without a database
    let state = build_state().build().await.unwrap();
    assert!(state.db().is_none());
}

#[tokio::test]
async fn builds_with_test_db() {
    let _state = build_state().with_db(DbProfile::Test).build().await.unwrap();
}

#[tokio::test]
async fn builds_with_test_db_skips_schema() {
    let _state = build_state()
        .with_db(DbProfile::Test)
        .assume_schema_ready()
        .build()
        .await
        .unwrap();
}
