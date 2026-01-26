use std::sync::Arc;

use sea_orm::DatabaseConnection;

use super::security_config::SecurityConfig;
use crate::config::email_allowlist::EmailAllowlist;
use crate::routes::snapshot_cache::SnapshotCache;
use crate::ws::hub::{RealtimeBroker, WsRegistry};

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
    /// WebSocket session registry.
    ///
    /// In production this is sourced from `realtime` (and is backed by Redis pub/sub fan-out).
    /// In tests it can be set directly to an in-memory registry to avoid Redis.
    pub(crate) websocket_registry: Option<Arc<WsRegistry>>,
    /// Snapshot cache for optimizing WebSocket broadcasts.
    ///
    /// Caches shared snapshot parts (game state, seating) to avoid redundant
    /// database queries when multiple users receive broadcasts for the same game version.
    pub snapshot_cache: Arc<SnapshotCache>,
}

impl AppState {
    fn new_inner(
        db: Option<DatabaseConnection>,
        security: SecurityConfig,
        email_allowlist: Option<EmailAllowlist>,
        realtime: Option<Arc<RealtimeBroker>>,
    ) -> Self {
        let websocket_registry = realtime.as_ref().map(|broker| broker.registry());
        let snapshot_cache = Arc::new(SnapshotCache::new());
        Self {
            db,
            security,
            email_allowlist,
            realtime,
            websocket_registry,
            snapshot_cache,
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
        self.websocket_registry = Some(realtime.registry());
        self.realtime = Some(realtime);
        self
    }

    /// Get a reference to the database connection if available
    pub fn db(&self) -> Option<&DatabaseConnection> {
        self.db.as_ref()
    }

    /// Attach a WebSocket session registry.
    ///
    /// Provides an in-memory registry without requiring Redis.
    pub fn with_websocket_registry(mut self, registry: Arc<WsRegistry>) -> Self {
        self.websocket_registry = Some(registry);
        self
    }

    /// Get the WebSocket session registry, if configured.
    pub fn websocket_registry(&self) -> Option<Arc<WsRegistry>> {
        self.websocket_registry.clone()
    }

    /// Get a reference to the snapshot cache.
    pub fn snapshot_cache(&self) -> &SnapshotCache {
        &self.snapshot_cache
    }

    /// Get an Arc clone of the snapshot cache.
    ///
    /// Useful when the cache needs to be moved into async closures.
    pub fn snapshot_cache_arc(&self) -> Arc<SnapshotCache> {
        self.snapshot_cache.clone()
    }
}
