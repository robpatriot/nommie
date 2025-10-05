use sea_orm::{Database, DatabaseConnection};

use crate::config::db::{db_url, DbOwner, DbProfile};
use crate::error::AppError;

/// Unified database connector that supports different profiles and owners
/// This function does NOT run any migrations
pub async fn connect_db(
    profile: DbProfile,
    owner: DbOwner,
) -> Result<DatabaseConnection, AppError> {
    // Build database URL from environment variables
    let database_url = db_url(profile, owner)?;

    // Connect to database
    let conn = Database::connect(&database_url).await?;
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
