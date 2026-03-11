use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "admin_audit_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(column_name = "actor_user_id")]
    pub actor_user_id: i64,
    pub action: String,
    #[sea_orm(column_name = "target_type")]
    pub target_type: String,
    #[sea_orm(column_name = "target_id")]
    pub target_id: i64,
    pub result: String,
    pub reason: Option<String>,
    #[sea_orm(column_name = "metadata_json")]
    pub metadata_json: Option<Json>,
    #[sea_orm(column_name = "created_at")]
    pub created_at: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::ActorUserId",
        to = "super::users::Column::Id"
    )]
    ActorUser,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ActorUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
