use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};

/// Schema guard for tests - ensures the test database has the required schema
/// This will panic with clear instructions if the schema is not prepared
pub async fn ensure_schema_ready(db: &DatabaseConnection) {
    // Check for the migrations table which should exist after running migrations
    let result = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT 1 FROM information_schema.tables WHERE table_name = 'seaql_migrations'",
        ))
        .await;

    match result {
        Ok(Some(_)) => {
            // Schema is ready, continue
        }
        Ok(None) | Err(_) => {
            panic!(
                "Schema not prepared. Run: `pnpm db:fresh:test` (Milestone D policy: tests never run migrations)."
            );
        }
    }
}
