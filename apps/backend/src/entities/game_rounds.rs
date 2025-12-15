use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "card_trump")]
pub enum CardTrump {
    #[sea_orm(string_value = "CLUBS")]
    Clubs,
    #[sea_orm(string_value = "DIAMONDS")]
    Diamonds,
    #[sea_orm(string_value = "HEARTS")]
    Hearts,
    #[sea_orm(string_value = "SPADES")]
    Spades,
    #[sea_orm(string_value = "NO_TRUMPS")]
    NoTrumps,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "game_rounds")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(column_name = "game_id")]
    pub game_id: i64,
    #[sea_orm(column_name = "round_no", column_type = "SmallInteger")]
    pub round_no: i16,
    #[sea_orm(column_name = "hand_size", column_type = "SmallInteger")]
    pub hand_size: i16,
    #[sea_orm(column_name = "dealer_pos", column_type = "SmallInteger")]
    pub dealer_pos: i16,
    pub trump: Option<CardTrump>,
    #[sea_orm(column_name = "created_at")]
    pub created_at: OffsetDateTime,
    #[sea_orm(column_name = "completed_at")]
    pub completed_at: Option<OffsetDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::games::Entity",
        from = "Column::GameId",
        to = "super::games::Column::Id"
    )]
    Game,
}

impl Related<super::games::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Game.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
