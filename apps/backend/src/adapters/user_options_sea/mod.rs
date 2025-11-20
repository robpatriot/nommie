//! SeaORM adapter for user options repository.

use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, Set};
use time::OffsetDateTime;

use crate::entities::user_options;

pub async fn find_by_user_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
) -> Result<Option<user_options::Model>, sea_orm::DbErr> {
    user_options::Entity::find_by_id(user_id).one(conn).await
}

pub async fn ensure_default_for_user(
    txn: &DatabaseTransaction,
    user_id: i64,
) -> Result<user_options::Model, sea_orm::DbErr> {
    if let Some(existing) = find_by_user_id(txn, user_id).await? {
        return Ok(existing);
    }

    let active = user_options::ActiveModel {
        user_id: Set(user_id),
        appearance_mode: Set("system".to_string()),
        updated_at: Set(OffsetDateTime::now_utc()),
    };

    match active.insert(txn).await {
        Ok(model) => Ok(model),
        Err(err) => {
            if let Some(existing) = find_by_user_id(txn, user_id).await? {
                Ok(existing)
            } else {
                Err(err)
            }
        }
    }
}

pub async fn set_appearance_mode(
    txn: &DatabaseTransaction,
    user_id: i64,
    appearance_mode: &str,
) -> Result<user_options::Model, sea_orm::DbErr> {
    let existing = ensure_default_for_user(txn, user_id).await?;
    let mut active: user_options::ActiveModel = existing.into();
    active.appearance_mode = Set(appearance_mode.to_string());
    active.updated_at = Set(OffsetDateTime::now_utc());
    active.update(txn).await
}
