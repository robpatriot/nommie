use sea_orm::DatabaseConnection;

use super::security_config::SecurityConfig;

/// Application state containing shared resources
#[derive(Debug, Clone)]
pub struct AppState {
    /// Database connection (optional for test scenarios)
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

    /// Create a new AppState without a database connection (for testing)
    pub fn without_db(security: SecurityConfig) -> Self {
        Self { db: None, security }
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

    /// Create a test AppState without database connection
    #[cfg(test)]
    pub fn for_tests_without_db() -> Self {
        Self::without_db(SecurityConfig::for_tests())
    }
}
