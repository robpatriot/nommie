use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use moka::future::Cache;
use once_cell::sync::OnceCell;
use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;
use tracing::{debug, info};
use xxhash_rust::xxh3::xxh3_64;

use crate::config::db::{
    build_connection_settings, make_conn_spec, ConnectionSettings, DbOwner, PoolPurpose,
};
use crate::error::AppError;
use crate::infra::db::core::build_pool;
use crate::infra::db::{DbKind, RuntimeEnv};
use crate::logging::pii::Redacted;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PoolKey {
    env: RuntimeEnv,
    db_kind: DbKind,
    db_url_hash: u64,
}

impl PoolKey {
    fn new(env: RuntimeEnv, db_kind: DbKind, db_url: &str) -> Self {
        Self {
            env,
            db_kind,
            db_url_hash: xxh3_64(db_url.as_bytes()),
        }
    }

    fn sanitized_log_key(&self, _db_url: &str) -> String {
        format!("{:?}:{:?}:{:x}", self.env, self.db_kind, self.db_url_hash)
    }
}

static SHARED_POOL_CACHE: OnceCell<Cache<PoolKey, Arc<DatabaseConnection>>> = OnceCell::new();
static INIT_LOCKS: OnceCell<DashMap<PoolKey, Arc<Mutex<()>>>> = OnceCell::new();

fn build_cache() -> Cache<PoolKey, Arc<DatabaseConnection>> {
    // Capacity-only LRU. Choose a conservative default that covers typical distinct keys.
    // No TTL/TTI. Eviction closes only when this was the last strong ref (via drop).
    Cache::builder()
        .max_capacity(64)
        .eviction_listener(|_k, v: Arc<DatabaseConnection>, _cause| {
            // Close only if cache held the final strong reference; otherwise do nothing.
            if Arc::strong_count(&v) == 1 {
                // Dropping the pool gracefully closes underlying connections.
                debug!("shared_pool=evicted action=drop_last_ref");
                drop(v);
            } else {
                debug!(
                    "shared_pool=evicted action=retain_external_refs count={}",
                    Arc::strong_count(&v)
                );
            }
        })
        .build()
}

async fn create_pool(
    env: RuntimeEnv,
    db_kind: DbKind,
    pool_cfg: &ConnectionSettings,
) -> Result<DatabaseConnection, AppError> {
    build_pool(env, db_kind, pool_cfg).await
}

pub async fn get_or_create_shared_pool(
    env: RuntimeEnv,
    db_kind: DbKind,
) -> Result<Arc<DatabaseConnection>, AppError> {
    let pool_cfg = build_connection_settings(env, db_kind, PoolPurpose::Runtime)?;
    let db_url = make_conn_spec(env, db_kind, DbOwner::App)?;
    let key = PoolKey::new(env, db_kind, &db_url);
    let sanitized_key = key.sanitized_log_key(&db_url);

    let cache = SHARED_POOL_CACHE.get_or_init(build_cache);

    // Fast path: outside the mutex
    if let Some(value) = cache.get(&key).await {
        debug!(
            shared_pool = "reuse",
            key = %Redacted(&sanitized_key),
            env = ?key.env,
            db_kind = ?key.db_kind,
            "Reusing existing shared database pool"
        );
        return Ok(value);
    }

    let init_locks = INIT_LOCKS.get_or_init(DashMap::new);
    let lock = {
        let maybe = init_locks.get(&key).map(|e| e.value().clone());
        match maybe {
            Some(l) => l,
            None => {
                let new_mutex = Arc::new(Mutex::new(()));
                init_locks.insert(key.clone(), new_mutex.clone());
                new_mutex
            }
        }
    };

    // Acquire per-key mutex
    let wait_start = Instant::now();
    let _guard = lock.lock().await;
    let dedup_wait_ms = wait_start.elapsed().as_millis();
    if dedup_wait_ms > 0 {
        debug!(
            shared_pool = "dedup_wait_ms",
            key = %Redacted(&sanitized_key),
            env = ?key.env,
            db_kind = ?key.db_kind,
            wait_ms = dedup_wait_ms,
            "Waited for concurrent pool creation"
        );
    }

    // Second check: inside the mutex
    if let Some(value) = cache.get(&key).await {
        debug!(
            shared_pool = "reuse",
            key = %Redacted(&sanitized_key),
            env = ?key.env,
            db_kind = ?key.db_kind,
            "Reusing pool created by concurrent task"
        );
        return Ok(value);
    }

    info!(
        "pool=about_to_build env={:?} db_kind={:?} owner=App",
        env, db_kind
    );
    let pool = create_pool(env, db_kind, &pool_cfg).await?;
    let arc_pool = Arc::new(pool);
    cache.insert(key.clone(), arc_pool.clone()).await;

    if let Some(locks) = INIT_LOCKS.get() {
        locks.remove(&key);
    }

    Ok(arc_pool)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_pool_key_equality_and_inequality() {
        let url_a = "postgresql://user:pass@localhost/db1";
        let url_a_copy = "postgresql://user:pass@localhost/db1";
        let url_b = "postgresql://user:pass@localhost/db2";

        let k1 = PoolKey::new(RuntimeEnv::Test, DbKind::Postgres, url_a);
        let k2 = PoolKey::new(RuntimeEnv::Test, DbKind::Postgres, url_a_copy);
        assert_eq!(k1, k2);

        let k3 = PoolKey::new(RuntimeEnv::Prod, DbKind::Postgres, url_a);
        assert_ne!(k1, k3);

        let k4 = PoolKey::new(RuntimeEnv::Test, DbKind::SqliteFile, url_a);
        assert_ne!(k1, k4);

        let k5 = PoolKey::new(RuntimeEnv::Test, DbKind::Postgres, url_b);
        assert_ne!(k1, k5);
    }

    #[tokio::test]
    async fn test_init_locks_single_flight_behavior() {
        // Build a synthetic key (no DB access needed)
        let key = PoolKey::new(RuntimeEnv::Test, DbKind::SqliteMemory, "sqlite::memory:");

        // Fresh map for this test run
        let locks = INIT_LOCKS.get_or_init(DashMap::new);

        // Acquire or insert a per-key mutex
        let lock1 = {
            let maybe = locks.get(&key).map(|e| e.value().clone());
            match maybe {
                Some(l) => l,
                None => {
                    let m = Arc::new(Mutex::new(()));
                    locks.insert(key.clone(), m.clone());
                    m
                }
            }
        };

        // Spawn two tasks that contend on the same mutex
        let l1 = lock1.clone();
        let l2 = lock1.clone();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<&'static str>();

        let tx1 = tx.clone();
        let t1 = tokio::spawn(async move {
            let _g = l1.lock().await;
            tx1.send("t1_acquired").unwrap();
            // Hold the lock briefly to ensure contention
            tokio::time::sleep(Duration::from_millis(50)).await;
            tx1.send("t1_released").unwrap();
        });

        let tx2 = tx.clone();
        let t2 = tokio::spawn(async move {
            // This should wait until t1 releases
            let _g = l2.lock().await;
            tx2.send("t2_acquired").unwrap();
        });

        // We expect the ordering: t1_acquired -> t1_released -> t2_acquired
        let first = rx.recv().await.unwrap();
        assert_eq!(first, "t1_acquired");
        let second = rx.recv().await.unwrap();
        assert_eq!(second, "t1_released");
        let third = rx.recv().await.unwrap();
        assert_eq!(third, "t2_acquired");

        t1.await.unwrap();
        t2.await.unwrap();

        // Cleanup the lock entry so this test does not pollute global state for other tests
        if let Some(map) = INIT_LOCKS.get() {
            map.remove(&key);
        }
    }
}
