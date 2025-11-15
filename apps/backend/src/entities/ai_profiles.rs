use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ai_profiles")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(column_name = "registry_name")]
    pub registry_name: String,
    #[sea_orm(column_name = "registry_version")]
    pub registry_version: String,
    pub variant: String,
    pub display_name: String,
    pub playstyle: Option<String>,
    pub difficulty: Option<i32>,
    pub config: Option<Json>,
    #[sea_orm(column_name = "memory_level")]
    pub memory_level: Option<i32>,
    #[sea_orm(column_name = "created_at")]
    pub created_at: OffsetDateTime,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::game_players::Entity")]
    GamePlayers,
}

impl Related<super::game_players::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GamePlayers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
