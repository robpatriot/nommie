use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_credentials")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(column_name = "user_id")]
    pub user_id: Uuid,
    #[sea_orm(column_name = "password_hash")]
    pub password_hash: Option<String>,
    pub email: String,
    #[sea_orm(column_name = "google_sub")]
    pub google_sub: Option<String>,
    #[sea_orm(column_name = "last_login")]
    pub last_login: Option<OffsetDateTime>,
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
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
