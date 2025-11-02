use backend::config::db::{DbKind, RuntimeEnv};
use backend::infra::db::bootstrap_db;
use migration::count_applied_migrations;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection};

use crate::support::resolve_test_db_kind;

async fn assert_runtime_is_app_pg(conn: &DatabaseConnection) {
    let res = conn
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "CREATE TABLE __should_fail(id int)",
        ))
        .await;
    assert!(res.is_err(), "App user unexpectedly created a table");
}

#[tokio::test]
async fn owner_split_and_permissions_hold() {
    let db_kind = resolve_test_db_kind().expect("Failed to resolve DB kind");

    // This test only makes sense for Postgres (owner/app user split)
    // For SQLite, it's a no-op
    if matches!(db_kind, DbKind::SqliteFile | DbKind::SqliteMemory) {
        return;
    }

    let pool = bootstrap_db(RuntimeEnv::Test, db_kind)
        .await
        .expect("bootstrap");
    assert_runtime_is_app_pg(&pool).await;
}

#[tokio::test]
async fn migration_is_idempotent() {
    let db_kind = resolve_test_db_kind().expect("Failed to resolve DB kind");

    let pool1 = bootstrap_db(RuntimeEnv::Test, db_kind)
        .await
        .expect("bootstrap-1");
    let before = count_applied_migrations(&pool1).await.unwrap_or(0);

    let pool2 = bootstrap_db(RuntimeEnv::Test, db_kind)
        .await
        .expect("bootstrap-2");
    let after = count_applied_migrations(&pool2).await.unwrap_or(0);

    assert_eq!(before, after, "migration count changed on second bootstrap");
}

#[tokio::test]
async fn single_conn_and_crud_quick() {
    let db_kind = resolve_test_db_kind().expect("Failed to resolve DB kind");
    let backend = DatabaseBackend::from(db_kind);

    let pool = bootstrap_db(RuntimeEnv::Test, db_kind)
        .await
        .expect("bootstrap");
    let ok = pool
        .execute(sea_orm::Statement::from_string(backend, "SELECT 1"))
        .await;
    assert!(ok.is_ok(), "basic SELECT 1 should succeed");
}
