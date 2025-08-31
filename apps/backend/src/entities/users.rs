use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub username: Option<String>,
    #[sea_orm(column_name = "is_ai")]
    pub is_ai: bool,
    #[sea_orm(column_name = "created_at")]
    pub created_at: OffsetDateTime,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_credentials::Entity")]
    UserCredentials,
}

impl Related<super::user_credentials::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserCredentials.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
