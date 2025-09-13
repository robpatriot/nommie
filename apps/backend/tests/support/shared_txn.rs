use std::sync::Arc;

use actix_web::{HttpMessage, HttpRequest};
use backend::db::txn::SharedTxn;
use sea_orm::{DatabaseConnection, TransactionTrait};

/// Begin a database transaction and wrap it as a SharedTxn.
///
/// Tests own the transaction lifecycle - this function does not commit or rollback.
pub async fn open(conn: &DatabaseConnection) -> SharedTxn {
    let txn = conn.begin().await.expect("Failed to begin transaction");
    SharedTxn(Arc::new(txn))
}

/// Inject a SharedTxn into the request extensions.
///
/// This allows the transaction to be used by with_txn() in handlers.
pub fn inject(req: &mut HttpRequest, shared: &SharedTxn) {
    req.extensions_mut().insert(shared.clone());
}

/// Rollback a shared transaction.
///
/// Tests own rollback - with_txn() must not auto-commit when SharedTxn is present.
pub async fn rollback(shared: SharedTxn) -> Result<(), sea_orm::DbErr> {
    let txn = Arc::try_unwrap(shared.0).map_err(|_| {
        sea_orm::DbErr::Custom("Cannot rollback: transaction is still shared".to_string())
    })?;
    txn.rollback().await
}
