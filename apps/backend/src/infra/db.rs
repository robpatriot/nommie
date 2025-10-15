use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::config::db::{db_url, DbOwner, DbProfile};
use crate::error::AppError;

/// Unified database connector that supports different profiles and owners
/// This function does NOT run any migrations
pub async fn connect_db(
    profile: DbProfile,
    owner: DbOwner,
) -> Result<DatabaseConnection, AppError> {
    match profile {
        DbProfile::InMemory => {
            // SQLite in-memory database
            let opt = ConnectOptions::new("sqlite::memory:");
            let conn = Database::connect(opt).await?;
            Ok(conn)
        }
        DbProfile::SqliteFile { file } => {
            // SQLite file-based database
            let db_path = if let Some(file) = file {
                // If a specific file is provided, use it as-is (could be absolute or relative)
                file
            } else {
                // Use default directory and filename
                let db_dir =
                    std::env::var("SQLITE_DB_DIR").unwrap_or_else(|_| "./data/sqlite".to_string());
                format!("{}/dev.db", db_dir)
            };

            // Debug logging
            tracing::info!("SQLite file path: {}", db_path);

            // Lazy directory creation (only for non-empty parent paths)
            if let Some(parent) = std::path::Path::new(&db_path).parent() {
                if !parent.as_os_str().is_empty() {
                    tracing::info!("Creating SQLite directory: {:?}", parent);
                    std::fs::create_dir_all(parent).map_err(|e| {
                        AppError::config(format!("Failed to create SQLite directory: {e}"))
                    })?;
                }
            }

            let url = format!("sqlite:{}?mode=rwc", db_path);
            tracing::info!("SQLite connection URL: {}", url);
            let conn = Database::connect(url).await.map_err(|e| {
                AppError::config(format!(
                    "Failed to connect to SQLite database at {}: {}",
                    db_path, e
                ))
            })?;

            Ok(conn)
        }
        DbProfile::Prod | DbProfile::Test => {
            // PostgreSQL databases (existing logic)
            let is_test = matches!(profile, DbProfile::Test);
            let database_url = db_url(profile, owner)?;

            let mut opt = ConnectOptions::new(&database_url);

            if is_test {
                opt.max_connections(10)
                    .min_connections(2)
                    .connect_timeout(Duration::from_secs(3))
                    .idle_timeout(Duration::from_secs(30))
                    .sqlx_logging(true);
            } else {
                opt.max_connections(20)
                    .min_connections(5)
                    .connect_timeout(Duration::from_secs(5))
                    .idle_timeout(Duration::from_secs(600))
                    .sqlx_logging(true);
            }

            let conn = Database::connect(opt).await?;
            Ok(conn)
        }
    }
}

/// Connect to database for migration purposes (always uses Owner privileges)
/// Target string should be one of: "prod", "pg_test", "sqlite_test"
pub async fn connect_db_for_migration(target: &str) -> Result<DatabaseConnection, AppError> {
    let profile = match target {
        "prod" => DbProfile::Prod,
        "pg_test" => DbProfile::Test,
        "sqlite_test" => DbProfile::SqliteFile { file: None },
        _ => {
            return Err(AppError::config(format!(
                "Invalid migration target '{}'. Must be one of: prod, pg_test, sqlite_test",
                target
            )));
        }
    };

    connect_db(profile, DbOwner::Owner).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_connect_db_signature() {
        // This test just ensures the function compiles with the new signature
        // The actual connection testing is done in integration tests
        // No-op test to verify compilation
    }
}
