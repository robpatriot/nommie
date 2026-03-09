//! Repository layer for user options.

use sea_orm::DatabaseTransaction;
use time::OffsetDateTime;

use crate::adapters::user_options_sea as adapter;
pub use crate::entities::user_options::{ColourScheme, Theme, UserLocale};
use crate::errors::domain::DomainError;

pub const DEFAULT_TRICK_DISPLAY_DURATION_SECONDS: f64 = 2.0;

#[derive(Debug, Clone, PartialEq)]
pub struct UserOptions {
    pub user_id: i64,
    pub colour_scheme: ColourScheme,
    pub theme: Theme,
    pub require_card_confirmation: bool,
    pub locale: Option<UserLocale>,
    pub trick_display_duration_seconds: Option<f64>,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct UpdateUserOptions {
    pub colour_scheme: Option<ColourScheme>,
    pub theme: Option<Theme>,
    pub require_card_confirmation: Option<bool>,
    // Option<Option<UserLocale>> allows distinguishing:
    // - None = field not provided (don't update)
    // - Some(None) = field provided as null (explicitly unset)
    // - Some(Some(locale)) = field provided with value
    pub locale: Option<Option<UserLocale>>,
    // Option<Option<f64>> allows distinguishing:
    // - None = field not provided (don't update)
    // - Some(None) = field provided as null (explicitly unset to default)
    // - Some(Some(value)) = field provided with value
    pub trick_display_duration_seconds: Option<Option<f64>>,
}

pub async fn ensure_default_for_user(
    txn: &DatabaseTransaction,
    user_id: i64,
) -> Result<UserOptions, DomainError> {
    let model = adapter::ensure_default_for_user(txn, user_id).await?;
    Ok(UserOptions::from(model))
}

pub async fn update_options(
    txn: &DatabaseTransaction,
    user_id: i64,
    options: UpdateUserOptions,
) -> Result<UserOptions, DomainError> {
    let model = adapter::update_options(
        txn,
        user_id,
        options.colour_scheme,
        options.theme,
        options.require_card_confirmation,
        options.locale,
        options.trick_display_duration_seconds,
    )
    .await?;
    Ok(UserOptions::from(model))
}

impl From<crate::entities::user_options::Model> for UserOptions {
    fn from(model: crate::entities::user_options::Model) -> Self {
        Self {
            user_id: model.user_id,
            colour_scheme: model.colour_scheme,
            theme: model.theme,
            require_card_confirmation: model.require_card_confirmation,
            locale: model.locale,
            trick_display_duration_seconds: model.trick_display_duration_seconds,
            updated_at: model.updated_at,
        }
    }
}
