use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use actix_web::{HttpMessage, HttpRequest};
use sea_orm::{DatabaseConnection, DatabaseTransaction, TransactionTrait};

use super::{require_db, txn_policy};
use crate::error::AppError;
use crate::state::app_state::AppState;

/// Global counter for active database transactions.
/// Used for monitoring connection pool usage and detecting transaction leaks.
pub(crate) static ACTIVE_TXNS: AtomicU64 = AtomicU64::new(0);

/// A shared transaction wrapper that can be injected into request extensions
#[derive(Clone)]
pub struct SharedTxn(pub Arc<DatabaseTransaction>);

impl SharedTxn {
    /// Begin a database transaction and wrap it as a SharedTxn.
    ///
    /// This method creates a new transaction that can be shared across multiple operations.
    /// The caller is responsible for managing the transaction lifecycle (commit/rollback).
    ///
    /// # Test-only utility
    /// This method is primarily used in integration tests. It cannot be marked `#[cfg(test)]`
    /// because integration tests are in a separate crate.
    pub async fn open(conn: &DatabaseConnection) -> Result<Self, sea_orm::DbErr> {
        let txn = conn.begin().await?;
        Ok(SharedTxn(Arc::new(txn)))
    }

    /// Get a reference to the underlying database transaction
    pub fn transaction(&self) -> &DatabaseTransaction {
        &self.0
    }

    /// Inject this SharedTxn into the request extensions.
    ///
    /// This allows the transaction to be used by with_txn() in handlers.
    ///
    /// # Test-only utility
    /// This method is primarily used in integration tests. It cannot be marked `#[cfg(test)]`
    /// because integration tests are in a separate crate.
    pub fn inject(&self, req: &mut HttpRequest) {
        req.extensions_mut().insert(self.clone());
    }

    /// Extract a SharedTxn from the request extensions.
    ///
    /// This is the symmetrical "get" partner to `inject`. Returns a cloned
    /// SharedTxn if present in the request extensions, otherwise None.
    pub fn from_req(req: &HttpRequest) -> Option<Self> {
        req.extensions().get::<SharedTxn>().cloned()
    }

    /// Rollback this shared transaction.
    ///
    /// The caller owns the transaction lifecycle - with_txn() will not auto-commit when SharedTxn is present.
    ///
    /// # Test-only utility
    /// This method is primarily used in integration tests. It cannot be marked `#[cfg(test)]`
    /// because integration tests are in a separate crate.
    pub async fn rollback(self) -> Result<(), sea_orm::DbErr> {
        let txn = Arc::try_unwrap(self.0).map_err(|_| {
            sea_orm::DbErr::Custom("Cannot rollback: transaction is still shared".to_string())
        })?;
        txn.rollback().await
    }

    /// Get the number of strong references to this shared transaction.
    ///
    /// This is primarily used in test teardown to wait for all clones to be dropped
    /// before attempting rollback.
    ///
    /// # Test-only utility
    /// This method is primarily used in integration tests. It cannot be marked `#[cfg(test)]`
    /// because integration tests are in a separate crate.
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }
}

/// Execute a closure with a database transaction.
///
/// Transaction precedence:
/// (1) If a `SharedTxn` is present in request extensions, **reuse it**; do **not** begin/commit/rollback.
/// (2) Otherwise, begin a transaction; on `Ok` apply `txn_policy` (commit or rollback-on-ok), on `Err` rollback.
pub async fn with_txn<R, F>(
    req: Option<&HttpRequest>,
    state: &AppState,
    f: F,
) -> Result<R, AppError>
where
    // The closure takes a borrowed transaction and returns a boxed future
    // whose lifetime is tied to that borrow (no 'static requirements).
    F: for<'a> FnOnce(
            &'a DatabaseTransaction,
        ) -> Pin<Box<dyn Future<Output = Result<R, AppError>> + Send + 'a>>
        + Send,
{
    // Extract any SharedTxn out of request extensions *before* awaiting to avoid holding a RefCell borrow.
    let shared_txn: Option<SharedTxn> =
        req.and_then(|r| r.extensions().get::<SharedTxn>().cloned());

    if let Some(shared) = shared_txn {
        // Use the provided shared transaction; no commit/rollback here.
        return f(shared.transaction()).await;
    }

    // Check if database is available
    let db = require_db(state)?;

    // Real DB path: own the transaction lifecycle
    let txn = db.begin().await?;

    // Increment active transaction counter
    ACTIVE_TXNS.fetch_add(1, Ordering::SeqCst);

    let out = f(&txn).await;

    match out {
        Ok(val) => {
            // Apply transaction policy on success
            match txn_policy::current() {
                txn_policy::TxnPolicy::CommitOnOk => {
                    txn.commit().await?;
                    ACTIVE_TXNS.fetch_sub(1, Ordering::SeqCst);
                    Ok(val)
                }
                _ => {
                    // RollbackOnOk (test-only policy) - rollback on success
                    txn.rollback().await?;
                    ACTIVE_TXNS.fetch_sub(1, Ordering::SeqCst);
                    Ok(val)
                }
            }
        }
        Err(err) => {
            // Best-effort rollback; preserve original error
            let _ = txn.rollback().await;
            ACTIVE_TXNS.fetch_sub(1, Ordering::SeqCst);
            Err(err)
        }
    }
}
