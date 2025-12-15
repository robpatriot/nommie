use std::sync::Arc;

use sea_orm::DatabaseConnection;

use super::security_config::SecurityConfig;
use crate::config::email_allowlist::EmailAllowlist;
use crate::ws::hub::RealtimeBroker;

/// Application state containing shared resources
pub struct AppState {
    /// Database connection (optional)
    db: Option<DatabaseConnection>,
    /// Security configuration including JWT settings
    pub security: SecurityConfig,
    /// Email allowlist for restricting signup and login (None = allowlist disabled)
    pub email_allowlist: Option<EmailAllowlist>,
    /// Realtime broker for websocket fan-out (optional in tests)
    pub realtime: Option<Arc<RealtimeBroker>>,
}

impl AppState {
    fn new_inner(
        db: Option<DatabaseConnection>,
        security: SecurityConfig,
        email_allowlist: Option<EmailAllowlist>,
        realtime: Option<Arc<RealtimeBroker>>,
    ) -> Self {
        Self {
            db,
            security,
            email_allowlist,
            realtime,
        }
    }

    /// Create a new AppState with the given database connection and security config
    pub fn new(
        db: DatabaseConnection,
        security: SecurityConfig,
        email_allowlist: Option<EmailAllowlist>,
    ) -> Self {
        Self::new_inner(Some(db), security, email_allowlist, None)
    }

    /// Create a new AppState with no database connection
    pub fn new_without_db(
        security: SecurityConfig,
        email_allowlist: Option<EmailAllowlist>,
    ) -> Self {
        Self::new_inner(None, security, email_allowlist, None)
    }

    /// Attach realtime broker after initialization
    pub fn with_realtime(mut self, realtime: Arc<RealtimeBroker>) -> Self {
        self.realtime = Some(realtime);
        self
    }

    /// Get a reference to the database connection if available
    pub fn db(&self) -> Option<&DatabaseConnection> {
        self.db.as_ref()
    }
}
