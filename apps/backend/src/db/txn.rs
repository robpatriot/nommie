use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use actix_web::{HttpMessage, HttpRequest};
use sea_orm::{DatabaseTransaction, TransactionTrait};

use super::txn_policy;
use crate::error::AppError;
use crate::state::app_state::AppState;

/// A shared transaction wrapper that can be injected into request extensions
#[derive(Clone)]
pub struct SharedTxn(pub Arc<DatabaseTransaction>);

impl SharedTxn {
    /// Get a reference to the underlying database transaction
    pub fn transaction(&self) -> &DatabaseTransaction {
        &self.0
    }
}

// Precedence:
// 1) If a SharedTxn is present in req.extensions(), reuse it (no commit/rollback here).
// 2) If MockStrict DB and no SharedTxn, panic with guidance.
// 3) Otherwise (real DB), open transaction and apply current txn_policy on Ok; rollback on Err.
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
    let shared_txn: Option<SharedTxn> = if let Some(r) = req {
        r.extensions().get::<SharedTxn>().cloned()
    } else {
        None
    };

    if let Some(shared) = shared_txn {
        // Use the provided shared transaction; no commit/rollback here.
        return f(shared.transaction()).await;
    }

    // Real DB path: own the transaction lifecycle
    let txn = state.db.begin().await?;
    let out = f(&txn).await;

    match out {
        Ok(val) => {
            // Apply transaction policy on success
            match txn_policy::current() {
                txn_policy::TxnPolicy::CommitOnOk => {
                    txn.commit().await?;
                    Ok(val)
                }
                txn_policy::TxnPolicy::RollbackOnOk => {
                    txn.rollback().await?;
                    Ok(val)
                }
            }
        }
        Err(err) => {
            // Best-effort rollback; preserve original error
            let _ = txn.rollback().await;
            Err(err)
        }
    }
}
