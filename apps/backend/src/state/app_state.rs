use sea_orm::DatabaseConnection;

use super::security_config::SecurityConfig;

/// Application state containing shared resources
#[derive(Debug)]
pub struct AppState {
    /// Database connection
    pub db: DatabaseConnection,
    /// Security configuration including JWT settings
    pub security: SecurityConfig,
}

impl AppState {
    /// Create a new AppState with the given database connection and security config
    pub fn new(db: DatabaseConnection, security: SecurityConfig) -> Self {
        Self { db, security }
    }
}
