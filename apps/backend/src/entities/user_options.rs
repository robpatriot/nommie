use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "colour_scheme")]
pub enum ColourScheme {
    #[sea_orm(string_value = "system")]
    System,
    #[sea_orm(string_value = "light")]
    Light,
    #[sea_orm(string_value = "dark")]
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
}

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "theme")]
pub enum Theme {
    #[default]
    #[sea_orm(string_value = "standard")]
    Standard,
    #[serde(rename = "high_roller")]
    #[sea_orm(string_value = "high_roller")]
    HighRoller,
    #[sea_orm(string_value = "oldtime")]
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
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "user_locale")]
pub enum UserLocale {
    #[serde(rename = "en-GB")]
    #[sea_orm(string_value = "en-GB")]
    EnGb,
    #[serde(rename = "fr-FR")]
    #[sea_orm(string_value = "fr-FR")]
    FrFr,
    #[serde(rename = "de-DE")]
    #[sea_orm(string_value = "de-DE")]
    DeDe,
    #[serde(rename = "es-ES")]
    #[sea_orm(string_value = "es-ES")]
    EsEs,
    #[serde(rename = "it-IT")]
    #[sea_orm(string_value = "it-IT")]
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
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_options")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub user_id: i64,
    #[sea_orm(column_name = "colour_scheme")]
    pub colour_scheme: ColourScheme,
    #[sea_orm(column_name = "theme")]
    pub theme: Theme,
    #[sea_orm(column_name = "require_card_confirmation")]
    pub require_card_confirmation: bool,
    #[sea_orm(column_name = "locale")]
    pub locale: Option<UserLocale>,
    #[sea_orm(column_name = "trick_display_duration_seconds")]
    pub trick_display_duration_seconds: Option<f64>,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_delete = "Cascade"
    )]
    Users,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
