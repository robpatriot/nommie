use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_options")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub user_id: i64,
    #[sea_orm(column_name = "colour_scheme")]
    pub colour_scheme: String,
    #[sea_orm(column_name = "theme")]
    pub theme: String,
    #[sea_orm(column_name = "require_card_confirmation")]
    pub require_card_confirmation: bool,
    #[sea_orm(column_name = "locale")]
    pub locale: Option<String>,
    #[sea_orm(column_name = "trick_display_duration_seconds")]
    pub trick_display_duration_seconds: Option<f64>,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_delete = "Cascade"
    )]
    Users,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
