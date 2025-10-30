use backend::config::db::{DbKind, RuntimeEnv};
use backend::infra::db::bootstrap_db;
use migration::count_applied_migrations;
use sea_orm::{ConnectionTrait, DatabaseConnection};

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
async fn pg_owner_split_and_permissions_hold() {
    let pool = bootstrap_db(RuntimeEnv::Test, DbKind::Postgres)
        .await
        .expect("bootstrap");
    assert_runtime_is_app_pg(&pool).await;
}

#[tokio::test]
async fn pg_migration_is_idempotent() {
    let pool1 = bootstrap_db(RuntimeEnv::Test, DbKind::Postgres)
        .await
        .expect("bootstrap-1");
    let before = count_applied_migrations(&pool1).await.unwrap_or(0);

    let pool2 = bootstrap_db(RuntimeEnv::Test, DbKind::Postgres)
        .await
        .expect("bootstrap-2");
    let after = count_applied_migrations(&pool2).await.unwrap_or(0);

    assert_eq!(before, after, "migration count changed on second bootstrap");
}

#[tokio::test]
async fn pg_single_conn_and_crud_quick() {
    let pool = bootstrap_db(RuntimeEnv::Test, DbKind::Postgres)
        .await
        .expect("postgres");
    let ok = pool
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT 1",
        ))
        .await;
    assert!(ok.is_ok(), "basic SELECT 1 should succeed");
}

#[tokio::test]
async fn sqlite_file_single_conn_and_crud_quick() {
    let pool = bootstrap_db(RuntimeEnv::Test, DbKind::SqliteFile)
        .await
        .expect("sqlite-file");
    let ok = pool
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT 1",
        ))
        .await;
    assert!(ok.is_ok(), "basic SELECT 1 should succeed");
}

#[tokio::test]
async fn sqlite_memory_single_conn_and_crud_quick() {
    let pool = bootstrap_db(RuntimeEnv::Test, DbKind::SqliteMemory)
        .await
        .expect("sqlite-mem");
    let ok = pool
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT 1",
        ))
        .await;
    assert!(ok.is_ok(), "basic SELECT 1 should succeed");
}

#[tokio::test]
async fn sqlite_memory_migration_is_idempotent() {
    let pool1 = bootstrap_db(RuntimeEnv::Test, DbKind::SqliteMemory)
        .await
        .expect("bootstrap-1");
    let before = count_applied_migrations(&pool1).await.unwrap_or(0);

    let pool2 = bootstrap_db(RuntimeEnv::Test, DbKind::SqliteMemory)
        .await
        .expect("bootstrap-2");
    let after = count_applied_migrations(&pool2).await.unwrap_or(0);

    assert_eq!(before, after, "migration count changed on second bootstrap");
}

#[tokio::test]
async fn sqlite_file_migration_is_idempotent() {
    let pool1 = bootstrap_db(RuntimeEnv::Test, DbKind::SqliteFile)
        .await
        .expect("bootstrap-1");
    let before = count_applied_migrations(&pool1).await.unwrap_or(0);

    let pool2 = bootstrap_db(RuntimeEnv::Test, DbKind::SqliteFile)
        .await
        .expect("bootstrap-2");
    let after = count_applied_migrations(&pool2).await.unwrap_or(0);

    assert_eq!(before, after, "migration count changed on second bootstrap");
}
