use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};

/// Schema guard - ensures the database has the required schema
/// In tests, this will panic if the schema is not prepared.
/// In production, this will log a warning and continue.
pub async fn ensure_schema_ready(db: &DatabaseConnection) {
    // Check for the migrations table and count existing migrations
    let result = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS cnt FROM seaql_migrations",
        ))
        .await;

    let migration_count = match result {
        Ok(Some(row)) => {
            // Try to parse the count, default to 0 if parsing fails
            row.try_get::<i64>("", "cnt").unwrap_or(0)
        }
        Ok(None) | Err(_) => {
            // Table missing or query error - treat as not ready
            0
        }
    };

    if migration_count == 0 {
        #[cfg(test)]
        {
            panic!("Test schema not prepared. Run: pnpm db:fresh:test or pnpm db:mig:test:refresh");
        }

        #[cfg(not(test))]
        {
            tracing::warn!(
                "Database schema not ready - no migrations found. Run: pnpm db:mig:refresh"
            );
        }
    }
}
