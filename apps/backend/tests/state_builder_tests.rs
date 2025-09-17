mod support;

use std::panic::AssertUnwindSafe;

use backend::infra::state::{build_state, ERR_MISSING_DB};
use futures_util::FutureExt;
use support::state_builder_ext::StateBuilderTestExt;

#[tokio::test]
async fn panic_when_no_db_selected() {
    let result = AssertUnwindSafe(build_state().build()).catch_unwind().await;

    match result {
        Ok(_) => panic!("Expected panic but got Ok"),
        Err(panic_payload) => {
            let panic_message = panic_payload
                .downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| panic_payload.downcast_ref::<&str>().copied())
                .unwrap_or("Unknown panic");

            assert_eq!(panic_message, ERR_MISSING_DB);
        }
    }
}

#[tokio::test]
async fn builds_with_mock_db() {
    let _state = build_state().with_mock_db().build().await.unwrap();
}

#[tokio::test]
async fn builds_with_existing_db_mock_skips_schema() {
    use sea_orm::{DatabaseBackend, MockDatabase};

    let mock_db = MockDatabase::new(DatabaseBackend::Postgres);
    let conn = mock_db.into_connection();

    let _state = build_state()
        .with_existing_db(conn)
        .assume_schema_ready()
        .build()
        .await
        .unwrap();
}
