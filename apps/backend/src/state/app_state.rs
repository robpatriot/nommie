use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::SecurityConfig;

/// Application state containing shared resources
#[derive(Debug, Clone)]
pub struct AppState {
    /// Database connection
    pub db: Arc<DatabaseConnection>,
    /// Security configuration including JWT settings
    pub security: Arc<SecurityConfig>,
}

impl AppState {
    /// Create a new AppState with the given database connection and security config
    pub fn new(db: DatabaseConnection, security: SecurityConfig) -> Self {
        Self {
            db: Arc::new(db),
            security: Arc::new(security),
        }
    }

    /// Create a test AppState with the given database connection and a random security config
    #[cfg(test)]
    pub fn for_tests(db: DatabaseConnection) -> Self {
        Self::new(db, SecurityConfig::for_tests())
    }

    /// Create a test AppState with the given database connection and security config
    #[cfg(test)]
    pub fn for_tests_with_security(db: DatabaseConnection, security: SecurityConfig) -> Self {
        Self::new(db, security)
    }
}
