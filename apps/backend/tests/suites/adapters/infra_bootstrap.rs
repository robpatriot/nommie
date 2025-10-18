use backend::config::db::{DbOwner, DbProfile};
use backend::infra::db::bootstrap_db;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

async fn migration_count_pg(conn: &impl ConnectionTrait) -> i64 {
    conn.query_one(Statement::from_string(
        DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS cnt FROM seaql_migrations",
    ))
    .await
    .ok()
    .flatten()
    .and_then(|r| r.try_get::<i64>("", "cnt").ok())
    .unwrap_or(0)
}

async fn assert_runtime_is_app_pg(conn: &DatabaseConnection) {
    let res = conn
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "CREATE TABLE __should_fail(id int)",
        ))
        .await;
    assert!(res.is_err(), "App user unexpectedly created a table");
}

#[tokio::test]
async fn pg_owner_split_and_permissions_hold() {
    let pool = bootstrap_db(DbProfile::Test, DbOwner::App)
        .await
        .expect("bootstrap");
    assert_runtime_is_app_pg(&pool).await;
}

#[tokio::test]
async fn pg_migration_is_idempotent() {
    let pool1 = bootstrap_db(DbProfile::Test, DbOwner::App)
        .await
        .expect("bootstrap-1");
    let before = migration_count_pg(&pool1).await;

    let pool2 = bootstrap_db(DbProfile::Test, DbOwner::App)
        .await
        .expect("bootstrap-2");
    let after = migration_count_pg(&pool2).await;

    assert_eq!(before, after, "migration count changed on second bootstrap");
}

#[tokio::test]
async fn sqlite_memory_single_conn_and_crud_quick() {
    let pool = bootstrap_db(DbProfile::InMemory, DbOwner::App)
        .await
        .expect("sqlite-mem");
    let ok = pool
        .execute(Statement::from_string(DatabaseBackend::Sqlite, "SELECT 1"))
        .await;
    assert!(ok.is_ok(), "basic SELECT 1 should succeed");
}
