use sea_orm::DatabaseConnection;

use super::security_config::SecurityConfig;
use crate::config::email_allowlist::EmailAllowlist;

/// Application state containing shared resources
pub struct AppState {
    /// Database connection (optional)
    db: Option<DatabaseConnection>,
    /// Security configuration including JWT settings
    pub security: SecurityConfig,
    /// Email allowlist for restricting signup and login (None = allowlist disabled)
    pub email_allowlist: Option<EmailAllowlist>,
}

impl AppState {
    fn new_inner(
        db: Option<DatabaseConnection>,
        security: SecurityConfig,
        email_allowlist: Option<EmailAllowlist>,
    ) -> Self {
        Self {
            db,
            security,
            email_allowlist,
        }
    }

    /// Create a new AppState with the given database connection and security config
    pub fn new(db: DatabaseConnection, security: SecurityConfig) -> Self {
        let email_allowlist = EmailAllowlist::from_env();
        Self::new_inner(Some(db), security, email_allowlist)
    }

    /// Create a new AppState with no database connection
    pub fn new_without_db(security: SecurityConfig) -> Self {
        let email_allowlist = EmailAllowlist::from_env();
        Self::new_inner(None, security, email_allowlist)
    }

    /// Get a reference to the database connection if available
    pub fn db(&self) -> Option<&DatabaseConnection> {
        self.db.as_ref()
    }
}
