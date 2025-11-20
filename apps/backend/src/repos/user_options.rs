//! Repository layer for user options.

use sea_orm::{ConnectionTrait, DatabaseTransaction};
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
    pub updated_at: OffsetDateTime,
}

pub async fn find_by_user_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
) -> Result<Option<UserOptions>, DomainError> {
    let model = adapter::find_by_user_id(conn, user_id).await?;
    model.map(UserOptions::try_from).transpose()
}

pub async fn ensure_default_for_user(
    txn: &DatabaseTransaction,
    user_id: i64,
) -> Result<UserOptions, DomainError> {
    let model = adapter::ensure_default_for_user(txn, user_id).await?;
    UserOptions::try_from(model)
}

pub async fn set_appearance_mode(
    txn: &DatabaseTransaction,
    user_id: i64,
    mode: AppearanceMode,
) -> Result<UserOptions, DomainError> {
    let model = adapter::set_appearance_mode(txn, user_id, mode.as_str()).await?;
    UserOptions::try_from(model)
}

impl TryFrom<crate::entities::user_options::Model> for UserOptions {
    type Error = DomainError;

    fn try_from(model: crate::entities::user_options::Model) -> Result<Self, Self::Error> {
        let appearance_mode = AppearanceMode::from_db(&model.appearance_mode, model.user_id)?;
        Ok(Self {
            user_id: model.user_id,
            appearance_mode,
            updated_at: model.updated_at,
        })
    }
}
