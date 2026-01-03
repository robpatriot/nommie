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
        require_card_confirmation: Set(true),
        locale: Set(None),
        trick_display_duration_seconds: Set(None),
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

pub async fn update_options(
    txn: &DatabaseTransaction,
    user_id: i64,
    appearance_mode: Option<&str>,
    require_card_confirmation: Option<bool>,
    // Option<Option<&str>> allows distinguishing:
    // - None = field not provided (don't update)
    // - Some(None) = field provided as null (explicitly unset to null)
    // - Some(Some(str)) = field provided with value
    locale: Option<Option<&str>>,
    // Option<Option<f64>> allows distinguishing:
    // - None = field not provided (don't update)
    // - Some(None) = field provided as null (explicitly unset to default)
    // - Some(Some(value)) = field provided with value
    trick_display_duration_seconds: Option<Option<f64>>,
) -> Result<user_options::Model, sea_orm::DbErr> {
    let existing = ensure_default_for_user(txn, user_id).await?;

    if appearance_mode.is_none()
        && require_card_confirmation.is_none()
        && locale.is_none()
        && trick_display_duration_seconds.is_none()
    {
        return Ok(existing);
    }

    let mut active: user_options::ActiveModel = existing.into();
    if let Some(mode) = appearance_mode {
        active.appearance_mode = Set(mode.to_string());
    }
    if let Some(require_confirmation) = require_card_confirmation {
        active.require_card_confirmation = Set(require_confirmation);
    }
    // Handle locale: Some(None) = set to null, Some(Some(str)) = set to value
    if let Some(locale_opt) = locale {
        active.locale = Set(locale_opt.map(|s| s.to_string()));
    }
    // Handle trick_display_duration_seconds: Some(None) = set to null, Some(Some(value)) = set to value
    if let Some(duration_opt) = trick_display_duration_seconds {
        active.trick_display_duration_seconds = Set(duration_opt);
    }
    active.updated_at = Set(OffsetDateTime::now_utc());
    active.update(txn).await
}
