//! Snapshot cache for optimizing WebSocket broadcasts.
//!
//! This module provides SnapshotCache which caches shared snapshot parts
//! to avoid redundant database queries when multiple users receive broadcasts.

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::Mutex;

use crate::domain::snapshot::{GameSnapshot, SeatPublic};
use crate::repos::games::Game;

/// Cached shared data for a game snapshot.
///
/// This struct holds all data that is the same for all users viewing the same game version.
#[derive(Debug, Clone)]
pub struct SharedSnapshotParts {
    pub game: Game,
    pub snapshot: GameSnapshot,
    pub seating: [SeatPublic; 4],
    pub version: i32,
}

/// Cache for shared snapshot parts, keyed by (game_id, version).
///
/// Uses DashMap for lock-free concurrent reads and per-key mutexes for deduplication
/// when multiple concurrent requests miss the cache.
pub struct SnapshotCache {
    cache: DashMap<(i64, i32), Arc<SharedSnapshotParts>>,
    init_locks: DashMap<(i64, i32), Arc<Mutex<()>>>,
}

impl SnapshotCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
            init_locks: DashMap::new(),
        }
    }

    /// Fast path: get cached value without locking.
    ///
    /// Returns None if not cached.
    pub fn get(&self, key: (i64, i32)) -> Option<Arc<SharedSnapshotParts>> {
        self.cache.get(&key).map(|entry| entry.value().clone())
    }

    /// Get cached value or insert a newly built value with deduplication.
    ///
    /// Uses double-checked locking pattern to prevent thundering herd:
    /// 1. Fast path: check cache (outside mutex)
    /// 2. If miss, acquire per-key mutex
    /// 3. Double-check cache (inside mutex)
    /// 4. If still miss, insert the provided value
    ///
    /// Returns the cached value (either existing or newly inserted).
    /// The builder should build the value before calling this method.
    pub async fn get_or_insert(
        &self,
        key: (i64, i32),
        value: SharedSnapshotParts,
    ) -> Arc<SharedSnapshotParts> {
        // Fast path: check cache outside mutex
        if let Some(cached) = self.cache.get(&key) {
            return cached.value().clone();
        }

        // Get or create per-key mutex
        let lock = {
            let maybe = self.init_locks.get(&key).map(|e| e.value().clone());
            match maybe {
                Some(l) => l,
                None => {
                    let new_mutex = Arc::new(Mutex::new(()));
                    self.init_locks.insert(key, new_mutex.clone());
                    new_mutex
                }
            }
        };

        // Acquire per-key mutex
        let _guard = lock.lock().await;

        // Double-check: cache might have been populated by concurrent task
        if let Some(cached) = self.cache.get(&key) {
            return cached.value().clone();
        }

        // Cache miss - insert the provided value
        let arc_value = Arc::new(value);
        self.cache.insert(key, arc_value.clone());

        // Clean up mutex (optional - can leave for reuse, but removing prevents memory leak)
        self.init_locks.remove(&key);

        arc_value
    }

    /// Remove a cached entry.
    ///
    /// Used for cache invalidation when game version increments.
    pub fn remove(&self, key: (i64, i32)) {
        self.cache.remove(&key);
        // Also clean up any associated mutex
        self.init_locks.remove(&key);
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        self.cache.clear();
        self.init_locks.clear();
    }
}

impl Default for SnapshotCache {
    fn default() -> Self {
        Self::new()
    }
}
