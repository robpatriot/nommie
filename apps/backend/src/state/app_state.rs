use sea_orm::DatabaseConnection;

use super::security_config::SecurityConfig;

/// Application state containing shared resources
pub struct AppState {
    /// Database connection (optional)
    db: Option<DatabaseConnection>,
    /// Security configuration including JWT settings
    pub security: SecurityConfig,
}

impl AppState {
    fn new_inner(db: Option<DatabaseConnection>, security: SecurityConfig) -> Self {
        Self { db, security }
    }

    /// Create a new AppState with the given database connection and security config
    pub fn new(db: DatabaseConnection, security: SecurityConfig) -> Self {
        Self::new_inner(Some(db), security)
    }

    /// Create a new AppState with no database connection
    pub fn new_without_db(security: SecurityConfig) -> Self {
        Self::new_inner(None, security)
    }

    /// Get a reference to the database connection if available
    pub fn db(&self) -> Option<&DatabaseConnection> {
        self.db.as_ref()
    }
}
