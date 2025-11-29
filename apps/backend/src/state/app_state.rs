use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::ws::hub::RealtimeBroker;

use super::security_config::SecurityConfig;

/// Application state containing shared resources
pub struct AppState {
    /// Database connection (optional)
    db: Option<DatabaseConnection>,
    /// Security configuration including JWT settings
    pub security: SecurityConfig,
    /// Realtime broker for websocket fan-out (optional in tests)
    pub realtime: Option<Arc<RealtimeBroker>>,
}

impl AppState {
    fn new_inner(
        db: Option<DatabaseConnection>,
        security: SecurityConfig,
        realtime: Option<Arc<RealtimeBroker>>,
    ) -> Self {
        Self {
            db,
            security,
            realtime,
        }
    }

    /// Create a new AppState with the given database connection and security config
    pub fn new(db: DatabaseConnection, security: SecurityConfig) -> Self {
        Self::new_inner(Some(db), security, None)
    }

    /// Create a new AppState with no database connection
    pub fn new_without_db(security: SecurityConfig) -> Self {
        Self::new_inner(None, security, None)
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
