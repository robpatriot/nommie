//! SeaORM adapter for admin_audit_log.

use sea_orm::{ActiveValue, DatabaseTransaction, EntityTrait, Set};

use crate::entities::admin_audit_log;

/// DTO for inserting an audit log entry.
#[derive(Debug, Clone)]
pub struct AuditEntryInsert {
    pub actor_user_id: i64,
    pub action: String,
    pub target_type: String,
    pub target_id: i64,
    pub result: String,
    pub reason: Option<String>,
    pub metadata_json: Option<serde_json::Value>,
}

pub async fn insert_entry(
    txn: &DatabaseTransaction,
    entry: AuditEntryInsert,
) -> Result<(), sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let active = admin_audit_log::ActiveModel {
        id: ActiveValue::NotSet,
        actor_user_id: Set(entry.actor_user_id),
        action: Set(entry.action),
        target_type: Set(entry.target_type),
        target_id: Set(entry.target_id),
        result: Set(entry.result),
        reason: Set(entry.reason),
        metadata_json: Set(entry.metadata_json),
        created_at: Set(now),
    };

    admin_audit_log::Entity::insert(active)
        .exec_without_returning(txn)
        .await?;

    Ok(())
}
