use std::fmt;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

use sea_orm::DatabaseConnection;
use tracing::info;

use super::security_config::SecurityConfig;
use crate::auth::google::GoogleVerifier;
use crate::config::db::{DbKind, RuntimeEnv};
use crate::readiness::ReadinessManager;
use crate::routes::snapshot_cache::SnapshotCache;
use crate::ws::hub::{RealtimeBroker, WsRegistry};

/// Simple wrapper to protect sensitive strings from accidental logging
#[derive(Clone)]
pub struct Secret<T>(pub T);

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<REDACTED>")
    }
}

/// Consolidated configuration DNA for the AppState
#[derive(Clone)]
pub struct AppConfig {
    pub env: RuntimeEnv,
    pub db_kind: DbKind,
    pub db_url: Secret<String>,
    pub redis_url: Secret<Option<String>>,
    pub security: SecurityConfig,
    pub google_verifier: GoogleVerifier,
}

/// Application state containing shared resources
pub struct AppState {
    /// Consolidated configuration
    pub config: AppConfig,
    /// Database connection (optional, can be updated via background recovery)
    db: Arc<RwLock<Option<DatabaseConnection>>>,
    /// Realtime broker (optional, can be updated via background recovery)
    realtime: Arc<RwLock<Option<Arc<RealtimeBroker>>>>,
    /// WebSocket session registry.
    websocket_registry: Arc<RwLock<Option<Arc<WsRegistry>>>>,
    /// Singleflight flags for resolution
    pub db_resolution_in_flight: AtomicBool,
    pub redis_resolution_in_flight: AtomicBool,
    /// Snapshot cache for optimizing WebSocket broadcasts.
    pub snapshot_cache: Arc<SnapshotCache>,
    /// Readiness state manager.
    readiness: Arc<ReadinessManager>,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        db: Option<DatabaseConnection>,
        realtime: Option<Arc<RealtimeBroker>>,
        readiness: Arc<ReadinessManager>,
    ) -> Self {
        let registry = realtime.as_ref().map(|broker| broker.registry());
        Self {
            config,
            db: Arc::new(RwLock::new(db)),
            realtime: Arc::new(RwLock::new(realtime)),
            websocket_registry: Arc::new(RwLock::new(registry)),
            db_resolution_in_flight: AtomicBool::new(false),
            redis_resolution_in_flight: AtomicBool::new(false),
            snapshot_cache: Arc::new(SnapshotCache::new()),
            readiness,
        }
    }

    /// Create a new AppState with no database connection (used primarily in tests)
    pub fn new_without_db(config: AppConfig, readiness: Option<Arc<ReadinessManager>>) -> Self {
        let readiness = readiness.unwrap_or_else(|| Arc::new(ReadinessManager::new()));
        Self::new(config, None, None, readiness)
    }

    /// Attach realtime broker (replaces existing if any)
    pub fn set_realtime(&self, realtime: Arc<RealtimeBroker>) {
        let new_registry = realtime.registry();
        let old_registry = if let Ok(mut reg) = self.websocket_registry.write() {
            reg.replace(new_registry.clone())
        } else {
            None
        };

        if let Some(old) = old_registry {
            if !Arc::ptr_eq(&old, &new_registry) {
                let closed = old.close_all_connections_now();
                if closed > 0 {
                    info!(
                        closed_connections = closed,
                        "realtime registry replaced; closed sessions from previous generation"
                    );
                }
            }
        }

        if let Ok(mut rt) = self.realtime.write() {
            *rt = Some(realtime);
        }
    }

    /// Attach database connection (replaces existing if any)
    pub fn set_db(&self, db: DatabaseConnection) {
        if let Ok(mut d) = self.db.write() {
            *d = Some(db);
        }
    }

    /// Get the security configuration.
    pub fn security(&self) -> &SecurityConfig {
        &self.config.security
    }

    /// Get a clone of the database connection if available
    ///
    /// Note: Returns an Option<DatabaseConnection> because SeaORM connections
    /// are internally Arcs and safe to clone.
    pub fn db(&self) -> Option<DatabaseConnection> {
        self.db.read().ok()?.clone()
    }

    /// Get the readiness manager.
    pub fn readiness(&self) -> &Arc<ReadinessManager> {
        &self.readiness
    }

    /// Attach a WebSocket session registry (replaces existing if any).
    pub fn set_websocket_registry(&self, registry: Arc<WsRegistry>) {
        if let Ok(mut reg) = self.websocket_registry.write() {
            *reg = Some(registry);
        }
    }

    /// Get the WebSocket session registry, if configured.
    pub fn websocket_registry(&self) -> Option<Arc<WsRegistry>> {
        self.websocket_registry.read().ok()?.clone()
    }

    /// Get the realtime broker, if configured.
    pub fn realtime(&self) -> Option<Arc<RealtimeBroker>> {
        self.realtime.read().ok()?.clone()
    }

    /// Get a reference to the snapshot cache.
    pub fn snapshot_cache(&self) -> &SnapshotCache {
        &self.snapshot_cache
    }

    /// Get an Arc clone of the snapshot cache.
    pub fn snapshot_cache_arc(&self) -> Arc<SnapshotCache> {
        self.snapshot_cache.clone()
    }
}
