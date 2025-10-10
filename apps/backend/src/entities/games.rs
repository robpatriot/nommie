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
    #[sea_orm(column_name = "starting_dealer_pos")]
    pub starting_dealer_pos: Option<i16>,
    #[sea_orm(column_name = "current_trick_no")]
    pub current_trick_no: i16,
    #[sea_orm(column_name = "current_round_id")]
    pub current_round_id: Option<i64>,
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

impl Model {
    /// Computes the current hand size based on the current round number.
    /// Returns None if current_round is None or out of valid range.
    pub fn hand_size(&self) -> Option<i16> {
        use crate::domain::rules;

        let round_no = self.current_round?;
        if !(1..=26).contains(&round_no) {
            return None;
        }

        rules::hand_size_for_round(round_no as u8).map(|hs| hs as i16)
    }

    /// Computes the current dealer position based on starting dealer and current round.
    /// Returns None if either starting_dealer_pos or current_round is None.
    pub fn dealer_pos(&self) -> Option<i16> {
        let starting = self.starting_dealer_pos?;
        let round = self.current_round?;

        // Dealer rotates each round: (starting_dealer + round_no - 1) % 4
        // Subtract 1 because round_no starts at 1, not 0
        Some((starting + (round - 1)) % 4)
    }
}
