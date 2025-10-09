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
    #[sea_orm(column_name = "user_id")]
    pub user_id: i64,
    #[sea_orm(column_name = "turn_order")]
    pub turn_order: i32,
    #[sea_orm(column_name = "is_ready")]
    pub is_ready: bool,
    #[sea_orm(column_name = "created_at")]
    pub created_at: OffsetDateTime,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::games::Entity",
        from = "Column::GameId",
        to = "super::games::Column::Id"
    )]
    Game,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::games::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Game.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
