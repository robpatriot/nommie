use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "game_state")]
pub enum GameState {
    #[sea_orm(string_value = "LOBBY")]
    Lobby,
    #[sea_orm(string_value = "DEALING")]
    Dealing,
    #[sea_orm(string_value = "BIDDING")]
    Bidding,
    #[sea_orm(string_value = "TRUMP_SELECTION")]
    TrumpSelection,
    #[sea_orm(string_value = "TRICK_PLAY")]
    TrickPlay,
    #[sea_orm(string_value = "SCORING")]
    Scoring,
    #[sea_orm(string_value = "BETWEEN_ROUNDS")]
    BetweenRounds,
    #[sea_orm(string_value = "COMPLETED")]
    Completed,
    #[sea_orm(string_value = "ABANDONED")]
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "game_visibility")]
pub enum GameVisibility {
    #[sea_orm(string_value = "PUBLIC")]
    Public,
    #[sea_orm(string_value = "PRIVATE")]
    Private,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "games")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(column_name = "created_by")]
    pub created_by: Option<i64>,
    pub visibility: GameVisibility,
    pub state: GameState,
    #[sea_orm(column_name = "created_at")]
    pub created_at: OffsetDateTime,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: OffsetDateTime,
    #[sea_orm(column_name = "started_at")]
    pub started_at: Option<OffsetDateTime>,
    #[sea_orm(column_name = "ended_at")]
    pub ended_at: Option<OffsetDateTime>,
    pub name: Option<String>,
    #[sea_orm(column_name = "join_code")]
    pub join_code: Option<String>,
    #[sea_orm(column_name = "rules_version")]
    pub rules_version: String,
    #[sea_orm(column_name = "rng_seed")]
    pub rng_seed: Option<i64>,
    #[sea_orm(column_name = "current_round")]
    pub current_round: Option<i16>,
    #[sea_orm(column_name = "hand_size")]
    pub hand_size: Option<i16>,
    #[sea_orm(column_name = "dealer_pos")]
    pub dealer_pos: Option<i16>,
    #[sea_orm(column_name = "lock_version")]
    pub lock_version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::CreatedBy",
        to = "super::users::Column::Id"
    )]
    User,
    #[sea_orm(has_many = "super::game_players::Entity")]
    GamePlayers,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::game_players::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GamePlayers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
