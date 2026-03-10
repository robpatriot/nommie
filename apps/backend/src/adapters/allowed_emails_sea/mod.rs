//! SeaORM adapter for allowed_emails (admission table).

use sea_orm::sea_query::OnConflict;
use sea_orm::{ConnectionTrait, DatabaseTransaction, EntityTrait, NotSet, Set};

use crate::entities::allowed_emails;

/// Allowlist rule: pattern (normalized email or wildcard) and admin flag.
#[derive(Debug, Clone)]
pub struct AllowRule {
    pub pattern: String,
    pub is_admin: bool,
}

pub async fn list_all<C: ConnectionTrait + Send + Sync>(
    conn: &C,
) -> Result<Vec<AllowRule>, sea_orm::DbErr> {
    let models = allowed_emails::Entity::find().all(conn).await?;
    Ok(models
        .into_iter()
        .map(|m| AllowRule {
            pattern: m.email,
            is_admin: m.is_admin,
        })
        .collect())
}

pub async fn insert_if_not_exists(
    txn: &DatabaseTransaction,
    email: &str,
    is_admin: bool,
) -> Result<bool, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let active = allowed_emails::ActiveModel {
        id: NotSet,
        email: Set(email.to_string()),
        is_admin: Set(is_admin),
        created_at: Set(now),
    };

    let result = allowed_emails::Entity::insert(active)
        .on_conflict(
            OnConflict::columns([allowed_emails::Column::Email])
                .do_nothing()
                .to_owned(),
        )
        .exec_without_returning(txn)
        .await?;

    Ok(result == 1)
}

/// Upsert exact normalized email as admin. Exact-email-only by contract.
/// Never marks wildcard rows as admin. Never creates duplicate normalized exact rows.
/// Atomic: safe for concurrent bootstrap across multiple processes.
pub async fn upsert_admin(
    txn: &DatabaseTransaction,
    normalized_email: &str,
) -> Result<(), sea_orm::DbErr> {
    if normalized_email.contains('*') {
        return Ok(());
    }

    let now = time::OffsetDateTime::now_utc();
    let active = allowed_emails::ActiveModel {
        id: NotSet,
        email: Set(normalized_email.to_string()),
        is_admin: Set(true),
        created_at: Set(now),
    };

    allowed_emails::Entity::insert(active)
        .on_conflict(
            OnConflict::columns([allowed_emails::Column::Email])
                .update_column(allowed_emails::Column::IsAdmin)
                .to_owned(),
        )
        .exec_without_returning(txn)
        .await?;

    Ok(())
}
