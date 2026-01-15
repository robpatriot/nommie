use backend::config::db::{DbKind, RuntimeEnv};
use backend::infra::db::bootstrap_db;
use db_infra::infra::db::build_admin_pool;
use migration::{Migrator, MigratorTrait};
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
#[ignore]
async fn owner_split_permissions_pg() {
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

    let _pool1 = bootstrap_db(RuntimeEnv::Test, db_kind)
        .await
        .expect("bootstrap-1");

    // Use admin pool for accurate migration count
    let before_admin_pool = build_admin_pool(RuntimeEnv::Test, db_kind)
        .await
        .expect("before admin pool");
    let before = Migrator::get_applied_migrations(&before_admin_pool)
        .await
        .unwrap_or_else(|_| vec![])
        .len();

    let _pool2 = bootstrap_db(RuntimeEnv::Test, db_kind)
        .await
        .expect("bootstrap-2");

    // Use admin pool for accurate migration count
    let after_admin_pool = build_admin_pool(RuntimeEnv::Test, db_kind)
        .await
        .expect("after admin pool");
    let after = Migrator::get_applied_migrations(&after_admin_pool)
        .await
        .unwrap_or_else(|_| vec![])
        .len();

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
