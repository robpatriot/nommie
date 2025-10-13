use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ai_overrides")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(column_name = "game_player_id")]
    pub game_player_id: i64,
    pub name: Option<String>,
    #[sea_orm(column_name = "memory_level")]
    pub memory_level: Option<i32>,
    pub config: Option<Json>,
    #[sea_orm(column_name = "created_at")]
    pub created_at: OffsetDateTime,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::game_players::Entity",
        from = "Column::GamePlayerId",
        to = "super::game_players::Column::Id"
    )]
    GamePlayer,
}

impl Related<super::game_players::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GamePlayer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
