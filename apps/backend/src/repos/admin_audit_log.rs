//! Admin audit log repository.

use sea_orm::DatabaseTransaction;

use crate::adapters::admin_audit_log_sea as adapter;
use crate::errors::domain::DomainError;

/// Insert an audit log entry. Caller must be in a transaction.
pub async fn insert_entry(
    txn: &DatabaseTransaction,
    entry: adapter::AuditEntryInsert,
) -> Result<(), DomainError> {
    adapter::insert_entry(txn, entry).await?;
    Ok(())
}
