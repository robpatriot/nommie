use std::sync::Arc;

use actix_web::{HttpMessage, HttpRequest};
use sea_orm::{DatabaseTransaction, TransactionTrait};

use crate::error::AppError;
use crate::infra::mock_strict;
use crate::state::app_state::AppState;

/// Error message for when MockStrict DB blocks a query because no shared test transaction was provided
pub const ERR_MOCK_STRICT_NO_SHARED_TXN: &str = "MockStrict DB blocked a query because no shared test transaction was provided. Either use .with_db(DbProfile::Test) for a real test DB, or inject a shared transaction into the request extensions (see tests/support/shared_txn.rs";

/// A shared transaction wrapper that can be injected into request extensions
#[derive(Clone)]
pub struct SharedTxn(pub Arc<DatabaseTransaction>);

impl SharedTxn {
    /// Get a reference to the underlying database transaction
    pub fn transaction(&self) -> &DatabaseTransaction {
        &self.0
    }
}

/// Execute a function within a database transaction
///
/// 1) If a SharedTxn is in request extensions → use it (no commit/rollback here)
/// 2) If using MockStrict DB without a SharedTxn → panic with guidance
/// 3) Otherwise (real DB) → begin txn, run closure, commit on Ok / rollback on Err
pub async fn with_txn<T, F, Fut>(
    req: Option<&HttpRequest>,
    state: &AppState,
    f: F,
) -> Result<T, AppError>
where
    F: FnOnce(&DatabaseTransaction) -> Fut,
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    // Extract any SharedTxn out of request extensions *before* awaiting to avoid holding a RefCell borrow.
    let shared_txn: Option<SharedTxn> = if let Some(r) = req {
        r.extensions().get::<SharedTxn>().cloned()
    } else {
        None
    };

    if let Some(shared) = shared_txn {
        return f(shared.transaction()).await;
    }

    // MockStrict without SharedTxn → block with exact guidance
    if mock_strict::is_mock_strict(&state.db) {
        panic!("{ERR_MOCK_STRICT_NO_SHARED_TXN}");
    }

    // Real DB path: own the transaction lifecycle
    let txn = state.db.begin().await?;
    let out = f(&txn).await;

    match out {
        Ok(val) => {
            txn.commit().await?;
            Ok(val)
        }
        Err(err) => {
            // Best-effort rollback; preserve original error
            let _ = txn.rollback().await;
            Err(err)
        }
    }
}
