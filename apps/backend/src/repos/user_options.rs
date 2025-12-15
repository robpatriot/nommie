//! Repository layer for user options.

use sea_orm::DatabaseTransaction;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::adapters::user_options_sea as adapter;
use crate::errors::domain::{DomainError, InfraErrorKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppearanceMode {
    System,
    Light,
    Dark,
}

impl AppearanceMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn from_db(value: &str, user_id: i64) -> Result<Self, DomainError> {
        match value {
            "system" => Ok(Self::System),
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            other => Err(DomainError::infra(
                InfraErrorKind::DataCorruption,
                format!("invalid appearance_mode '{other}' stored for user_id={user_id}"),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserOptions {
    pub user_id: i64,
    pub appearance_mode: AppearanceMode,
    pub require_card_confirmation: bool,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UpdateUserOptions {
    pub appearance_mode: Option<AppearanceMode>,
    pub require_card_confirmation: Option<bool>,
}

pub async fn ensure_default_for_user(
    txn: &DatabaseTransaction,
    user_id: i64,
) -> Result<UserOptions, DomainError> {
    let model = adapter::ensure_default_for_user(txn, user_id).await?;
    UserOptions::try_from(model)
}

pub async fn update_options(
    txn: &DatabaseTransaction,
    user_id: i64,
    options: UpdateUserOptions,
) -> Result<UserOptions, DomainError> {
    let model = adapter::update_options(
        txn,
        user_id,
        options.appearance_mode.map(|mode| mode.as_str()),
        options.require_card_confirmation,
    )
    .await?;
    UserOptions::try_from(model)
}

impl TryFrom<crate::entities::user_options::Model> for UserOptions {
    type Error = DomainError;

    fn try_from(model: crate::entities::user_options::Model) -> Result<Self, Self::Error> {
        let appearance_mode = AppearanceMode::from_db(&model.appearance_mode, model.user_id)?;
        Ok(Self {
            user_id: model.user_id,
            appearance_mode,
            require_card_confirmation: model.require_card_confirmation,
            updated_at: model.updated_at,
        })
    }
}
