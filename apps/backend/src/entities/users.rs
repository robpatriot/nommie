use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "user_role")]
pub enum UserRole {
    #[sea_orm(string_value = "user")]
    User,
    #[sea_orm(string_value = "admin")]
    Admin,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub username: Option<String>,
    #[sea_orm(column_name = "is_ai")]
    pub is_ai: bool,
    #[sea_orm(column_name = "role")]
    pub role: UserRole,
    #[sea_orm(column_name = "created_at")]
    pub created_at: OffsetDateTime,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_auth_identities::Entity")]
    UserAuthIdentities,
    #[sea_orm(has_many = "super::games::Entity")]
    Games,
    #[sea_orm(has_one = "super::user_options::Entity")]
    UserOptions,
}

impl Related<super::user_auth_identities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserAuthIdentities.def()
    }
}

impl Related<super::games::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Games.def()
    }
}

impl Related<super::user_options::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserOptions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
