//! Repository layer for user options.

use sea_orm::DatabaseTransaction;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::adapters::user_options_sea as adapter;
use crate::errors::domain::{DomainError, InfraErrorKind};

pub const DEFAULT_TRICK_DISPLAY_DURATION_SECONDS: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColourScheme {
    System,
    Light,
    Dark,
}

impl ColourScheme {
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
                format!("invalid colour_scheme '{other}' stored for user_id={user_id}"),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Standard,
    #[serde(rename = "high_roller")]
    HighRoller,
    Oldtime,
}

impl Theme {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::HighRoller => "high_roller",
            Self::Oldtime => "oldtime",
        }
    }

    pub fn from_db(value: &str, user_id: i64) -> Result<Self, DomainError> {
        match value {
            "standard" => Ok(Self::Standard),
            "high_roller" => Ok(Self::HighRoller),
            "oldtime" => Ok(Self::Oldtime),
            other => Err(DomainError::infra(
                InfraErrorKind::DataCorruption,
                format!("invalid theme '{other}' stored for user_id={user_id}"),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserLocale {
    #[serde(rename = "en-GB")]
    EnGb,
    #[serde(rename = "fr-FR")]
    FrFr,
    #[serde(rename = "de-DE")]
    DeDe,
    #[serde(rename = "es-ES")]
    EsEs,
    #[serde(rename = "it-IT")]
    ItIt,
}

impl UserLocale {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::EnGb => "en-GB",
            Self::FrFr => "fr-FR",
            Self::DeDe => "de-DE",
            Self::EsEs => "es-ES",
            Self::ItIt => "it-IT",
        }
    }

    pub fn from_db(value: &str, user_id: i64) -> Result<Self, DomainError> {
        match value {
            "en-GB" => Ok(Self::EnGb),
            "fr-FR" => Ok(Self::FrFr),
            "de-DE" => Ok(Self::DeDe),
            "es-ES" => Ok(Self::EsEs),
            "it-IT" => Ok(Self::ItIt),
            other => Err(DomainError::infra(
                InfraErrorKind::DataCorruption,
                format!("invalid locale '{other}' stored for user_id={user_id}"),
            )),
        }
    }
}

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
    UserOptions::try_from(model)
}

pub async fn update_options(
    txn: &DatabaseTransaction,
    user_id: i64,
    options: UpdateUserOptions,
) -> Result<UserOptions, DomainError> {
    // Convert Option<Option<UserLocale>> to Option<Option<&str>> for the adapter
    let locale_for_adapter = options.locale.map(|opt| opt.map(|l| l.as_str()));

    let model = adapter::update_options(
        txn,
        user_id,
        options.colour_scheme.map(|mode| mode.as_str()),
        options.theme.map(|t| t.as_str()),
        options.require_card_confirmation,
        locale_for_adapter,
        options.trick_display_duration_seconds,
    )
    .await?;
    UserOptions::try_from(model)
}

impl TryFrom<crate::entities::user_options::Model> for UserOptions {
    type Error = DomainError;

    fn try_from(model: crate::entities::user_options::Model) -> Result<Self, Self::Error> {
        let colour_scheme = ColourScheme::from_db(&model.colour_scheme, model.user_id)?;
        let theme = Theme::from_db(&model.theme, model.user_id)?;
        let locale = model
            .locale
            .map(|l| UserLocale::from_db(&l, model.user_id))
            .transpose()?;
        Ok(Self {
            user_id: model.user_id,
            colour_scheme,
            theme,
            require_card_confirmation: model.require_card_confirmation,
            locale,
            trick_display_duration_seconds: model.trick_display_duration_seconds,
            updated_at: model.updated_at,
        })
    }
}
