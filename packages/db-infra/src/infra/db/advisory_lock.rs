//! Shared advisory lock acquisition with retry, backoff, and optional fast path.
//! Used by migration and AI profiles bootstrap operations.

use std::sync::Arc;
use std::time::{Duration, Instant};

use rand::Rng;
use sea_orm::DatabaseConnection;
use tokio_util::sync::CancellationToken;
use tracing::trace;

use crate::config::db::{make_conn_spec, sanitize_db_url, DbOwner, RuntimeEnv};
use crate::error::DbInfraError;
use crate::infra::db::locking::{Guard, PgAdvisoryLock};

/// Result of attempting to acquire the bootstrap lock.
#[derive(Debug)]
pub enum AcquireResult {
    /// Lock acquired; caller must release the guard when done.
    Acquired(Guard),
    /// Fast path passed; no lock needed, work is already done.
    Skipped,
}

/// Optional callbacks for lock acquisition events (e.g. migration counters).
/// All callbacks are optional; pass `None` for each to disable.
#[derive(Default, Clone)]
pub struct LockCallbacks {
    /// Called when lock is acquired, with number of attempts.
    pub on_acquired: Option<Arc<dyn Fn(u32) + Send + Sync>>,
    /// Called on each backoff iteration.
    pub on_backoff: Option<Arc<dyn Fn() + Send + Sync>>,
    /// Called when acquisition times out.
    pub on_timeout: Option<Arc<dyn Fn() + Send + Sync>>,
    /// Called when lock is acquired, with total attempts for metrics.
    pub on_add_attempts: Option<Arc<dyn Fn(usize) + Send + Sync>>,
}

fn lock_acquire_timeout_ms(env: RuntimeEnv) -> u64 {
    std::env::var("NOMMIE_BOOTSTRAP_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(match env {
            RuntimeEnv::Test => 3000,
            _ => 900,
        })
}

fn build_lock_key(env: RuntimeEnv) -> Result<String, DbInfraError> {
    let url = make_conn_spec(env, DbOwner::Owner)?;
    let sanitized = sanitize_db_url(&url);
    Ok(format!("nommie:bootstrap:{:?}:{}", env, sanitized))
}

/// Acquire the bootstrap advisory lock with retry, backoff, and optional fast path.
///
/// - **fast_path**: When provided, called at the start of each retry iteration.
///   If it returns `Ok(true)`, acquisition is skipped and `Skipped` is returned.
/// - **cancellation_token**: When provided, allows cancellation during backoff.
/// - **callbacks**: Optional hooks for metrics (e.g. migration counters).
pub async fn acquire_bootstrap_lock<F, Fut, E>(
    pool: &DatabaseConnection,
    env: RuntimeEnv,
    fast_path: Option<F>,
    cancellation_token: Option<CancellationToken>,
    callbacks: LockCallbacks,
) -> Result<AcquireResult, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<bool, E>>,
    E: From<DbInfraError>,
{
    let lock_key = build_lock_key(env)?;
    let lock_acquire_ms = lock_acquire_timeout_ms(env);
    let mut lock = PgAdvisoryLock::new(pool.clone(), &lock_key);

    let start = Instant::now();
    let mut attempts: u32 = 0;

    loop {
        attempts += 1;

        if let Some(ref fp) = fast_path {
            if fp().await? {
                return Ok(AcquireResult::Skipped);
            }
        }

        if let Some(guard) = lock.try_acquire().await? {
            if let Some(ref cb) = callbacks.on_acquired {
                cb(attempts);
            }
            if let Some(ref cb) = callbacks.on_add_attempts {
                cb(attempts as usize);
            }
            trace!(
                lock = "won",
                operation = "bootstrap",
                env = ?env,
                attempts = attempts,
                elapsed_ms = start.elapsed().as_millis()
            );
            return Ok(AcquireResult::Acquired(guard));
        }

        let base_delay_ms = (5u64 << attempts.saturating_sub(1)).min(80);
        let jitter_ms = rand::rng().random::<u64>() % 4;
        let delay_ms = base_delay_ms + jitter_ms;

        trace!(
            lock = "backoff",
            operation = "bootstrap",
            env = ?env,
            attempts = attempts,
            delay_ms = delay_ms,
            elapsed_ms = start.elapsed().as_millis()
        );
        if let Some(ref cb) = callbacks.on_backoff {
            cb();
        }

        let sleep_fut = tokio::time::sleep(Duration::from_millis(delay_ms));

        match &cancellation_token {
            Some(token) => {
                tokio::select! {
                    _ = sleep_fut => {}
                    _ = token.cancelled() => {
                        return Err(DbInfraError::Config {
                            message: format!(
                                "bootstrap lock acquisition cancelled during backoff after {}ms",
                                start.elapsed().as_millis()
                            ),
                        }.into());
                    }
                }
            }
            None => {
                sleep_fut.await;
            }
        }

        if start.elapsed() >= Duration::from_millis(lock_acquire_ms) {
            if let Some(ref cb) = callbacks.on_timeout {
                cb();
            }
            return Err(DbInfraError::Config {
                message: format!(
                    "bootstrap lock acquisition timeout after {:?} ({} attempts)",
                    start.elapsed(),
                    attempts
                ),
            }
            .into());
        }
    }
}
