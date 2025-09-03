use crate::error::AppError;
use sea_orm::{Database, DatabaseConnection};
use std::env;

/// Database profile enum for different environments
#[derive(Debug, Clone, PartialEq)]
pub enum DbProfile {
    /// Production database profile
    Prod,
    /// Test database profile - enforces safety rules
    Test,
}

/// Unified database connector that supports different profiles
/// This function does NOT run any migrations
pub async fn connect_db(profile: DbProfile) -> Result<DatabaseConnection, AppError> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Read DATABASE_URL from environment
    let database_url = env::var("DATABASE_URL")?;

    // For Test profile, enforce safety rule: DB name must end with "_test"
    if profile == DbProfile::Test {
        validate_test_database_url(&database_url)?;
    }

    // Connect to database
    let conn = Database::connect(&database_url).await?;
    Ok(conn)
}

/// Validates that a test database URL targets a database with name ending in "_test"
/// This is a safety guard to prevent accidental operations on production databases
fn validate_test_database_url(database_url: &str) -> Result<(), AppError> {
    // Extract the database name from the URL
    // For PostgreSQL URLs like: postgresql://user:pass@host:port/dbname
    if let Some(db_name_start) = database_url.rfind('/') {
        let db_name = &database_url[db_name_start + 1..];

        // Remove any query parameters (e.g., ?sslmode=require)
        let db_name = db_name.split('?').next().unwrap_or(db_name);

        if !db_name.ends_with("_test") {
            return Err(AppError::config(format!(
                "Test profile requires database name to end with '_test', but got: '{db_name}'"
            )));
        }
    } else {
        return Err(AppError::config(format!(
            "Invalid database URL format: '{database_url}'"
        )));
    }

    Ok(())
}

/// Run database migrations (idempotent)
/// This should only be called from migration scripts (pnpm db:migrate), never from main.rs or tests
pub async fn run_migrations(conn: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    use migration::{Migrator, MigratorTrait};

    Migrator::up(conn, None).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_test_database_url_valid() {
        let valid_urls = vec![
            "postgresql://user:pass@localhost:5432/myapp_test",
            "postgresql://user:pass@localhost:5432/myapp_test?sslmode=require",
            "postgres://user:pass@localhost:5432/myapp_test",
            "postgresql://user@localhost:5432/myapp_test",
            "postgresql://localhost:5432/myapp_test",
        ];

        for url in valid_urls {
            assert!(
                validate_test_database_url(url).is_ok(),
                "URL should be valid: {url}"
            );
        }
    }

    #[test]
    fn test_validate_test_database_url_invalid() {
        let invalid_urls = vec![
            "postgresql://user:pass@localhost:5432/myapp_prod",
            "postgresql://user:pass@localhost:5432/myapp",
            "postgresql://user:pass@localhost:5432/production",
            "postgresql://user:pass@localhost:5432/myapp_test_backup",
            "postgresql://user:pass@localhost:5432/test_myapp",
        ];

        for url in invalid_urls {
            assert!(
                validate_test_database_url(url).is_err(),
                "URL should be invalid: {url}"
            );
        }
    }

    #[test]
    fn test_validate_test_database_url_malformed() {
        let malformed_urls = vec![
            "not-a-url",
            "postgresql://user:pass@localhost:5432",
            "postgresql://user:pass@localhost",
        ];

        for url in malformed_urls {
            assert!(
                validate_test_database_url(url).is_err(),
                "URL should be malformed: {url}"
            );
        }
    }

    #[test]
    fn test_db_profile_enum() {
        assert_eq!(DbProfile::Prod, DbProfile::Prod);
        assert_eq!(DbProfile::Test, DbProfile::Test);
        assert_ne!(DbProfile::Prod, DbProfile::Test);
    }
}
