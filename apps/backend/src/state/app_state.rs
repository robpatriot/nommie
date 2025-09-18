use sea_orm::DatabaseConnection;

use super::security_config::SecurityConfig;

/// Application state containing shared resources
#[derive(Debug)]
pub struct AppState {
    /// Database connection (optional)
    pub db: Option<DatabaseConnection>,
    /// Security configuration including JWT settings
    pub security: SecurityConfig,
}

impl AppState {
    /// Create a new AppState with the given database connection and security config
    pub fn new(db: DatabaseConnection, security: SecurityConfig) -> Self {
        Self {
            db: Some(db),
            security,
        }
    }

    /// Create a new AppState with no database connection
    pub fn new_without_db(security: SecurityConfig) -> Self {
        Self { db: None, security }
    }

    /// Get a reference to the database connection if available
    pub fn db(&self) -> Option<&DatabaseConnection> {
        self.db.as_ref()
    }
}
