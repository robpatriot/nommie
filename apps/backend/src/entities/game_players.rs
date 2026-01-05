use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "game_players")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(column_name = "game_id")]
    pub game_id: i64,
    #[sea_orm(column_name = "player_kind")]
    pub player_kind: PlayerKind,
    #[sea_orm(column_name = "human_user_id")]
    pub human_user_id: Option<i64>,
    #[sea_orm(column_name = "ai_profile_id")]
    pub ai_profile_id: Option<i64>,
    #[sea_orm(column_name = "turn_order", column_type = "SmallInteger")]
    pub turn_order: Option<i16>,
    #[sea_orm(column_name = "is_ready")]
    pub is_ready: bool,
    #[sea_orm(column_name = "role")]
    pub role: GameRole,
    #[sea_orm(column_name = "created_at")]
    pub created_at: OffsetDateTime,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::HumanUserId",
        to = "super::users::Column::Id"
    )]
    HumanUser,
    #[sea_orm(
        belongs_to = "super::games::Entity",
        from = "Column::GameId",
        to = "super::games::Column::Id"
    )]
    Game,
    #[sea_orm(
        belongs_to = "super::ai_profiles::Entity",
        from = "Column::AiProfileId",
        to = "super::ai_profiles::Column::Id"
    )]
    AiProfile,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::HumanUser.def()
    }
}

impl Related<super::games::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Game.def()
    }
}

impl Related<super::ai_profiles::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AiProfile.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum PlayerKind {
    #[sea_orm(string_value = "human")]
    Human,
    #[sea_orm(string_value = "ai")]
    Ai,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum GameRole {
    #[sea_orm(string_value = "player")]
    Player,
    #[sea_orm(string_value = "spectator")]
    Spectator,
}
