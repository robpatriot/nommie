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
    // Check profile before consuming it
    let is_test = matches!(profile, DbProfile::Test);

    // Build database URL from environment variables
    let database_url = db_url(profile, owner)?;

    // Configure connection options
    let mut opt = ConnectOptions::new(&database_url);

    // For test profile, use smaller pool but keep query logging for diagnostics
    if is_test {
        opt.max_connections(10)
            .min_connections(2)
            .connect_timeout(Duration::from_secs(3))
            .idle_timeout(Duration::from_secs(30))
            .sqlx_logging(true); // Keep query logging in tests - valuable for debugging
    } else {
        // Production: larger pool with query logging
        opt.max_connections(20)
            .min_connections(5)
            .connect_timeout(Duration::from_secs(5))
            .idle_timeout(Duration::from_secs(600))
            .sqlx_logging(true);
    }

    // Connect to database
    let conn = Database::connect(opt).await?;
    Ok(conn)
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
