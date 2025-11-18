use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "card_suit")]
pub enum CardSuit {
    #[sea_orm(string_value = "CLUBS")]
    Clubs,
    #[sea_orm(string_value = "DIAMONDS")]
    Diamonds,
    #[sea_orm(string_value = "HEARTS")]
    Hearts,
    #[sea_orm(string_value = "SPADES")]
    Spades,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "round_tricks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(column_name = "round_id")]
    pub round_id: i64,
    #[sea_orm(column_name = "trick_no", column_type = "SmallInteger")]
    pub trick_no: i16,
    #[sea_orm(column_name = "lead_suit")]
    pub lead_suit: CardSuit,
    #[sea_orm(column_name = "winner_seat", column_type = "SmallInteger")]
    pub winner_seat: i16,
    #[sea_orm(column_name = "created_at")]
    pub created_at: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::game_rounds::Entity",
        from = "Column::RoundId",
        to = "super::game_rounds::Column::Id"
    )]
    GameRound,
}

impl Related<super::game_rounds::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GameRound.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
