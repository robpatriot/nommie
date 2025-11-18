use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "round_scores")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(column_name = "round_id")]
    pub round_id: i64,
    #[sea_orm(column_name = "player_seat", column_type = "SmallInteger")]
    pub player_seat: i16,
    #[sea_orm(column_name = "bid_value", column_type = "SmallInteger")]
    pub bid_value: i16,
    #[sea_orm(column_name = "tricks_won", column_type = "SmallInteger")]
    pub tricks_won: i16,
    #[sea_orm(column_name = "bid_met")]
    pub bid_met: bool,
    #[sea_orm(column_name = "base_score", column_type = "SmallInteger")]
    pub base_score: i16,
    #[sea_orm(column_name = "bonus", column_type = "SmallInteger")]
    pub bonus: i16,
    #[sea_orm(column_name = "round_score", column_type = "SmallInteger")]
    pub round_score: i16,
    #[sea_orm(column_name = "total_score_after")]
    pub total_score_after: i16,
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
