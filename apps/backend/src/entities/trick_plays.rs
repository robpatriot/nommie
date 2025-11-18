use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "trick_plays")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(column_name = "trick_id")]
    pub trick_id: i64,
    #[sea_orm(column_name = "player_seat", column_type = "SmallInteger")]
    pub player_seat: i16,
    pub card: Json,
    #[sea_orm(column_name = "play_order", column_type = "SmallInteger")]
    pub play_order: i16,
    #[sea_orm(column_name = "played_at")]
    pub played_at: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::round_tricks::Entity",
        from = "Column::TrickId",
        to = "super::round_tricks::Column::Id"
    )]
    RoundTrick,
}

impl Related<super::round_tricks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RoundTrick.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
