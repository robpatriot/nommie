//! SeaORM adapter for AI profiles.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, NotSet,
    QueryFilter, Set,
};

use crate::entities::ai_profiles;

pub mod dto;

pub use dto::{AiProfileCreate, AiProfileUpdate};

// Adapter functions return DbErr; repos layer maps to DomainError via From<DbErr>.

pub async fn create_profile(
    txn: &DatabaseTransaction,
    dto: AiProfileCreate,
) -> Result<ai_profiles::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let profile_active = ai_profiles::ActiveModel {
        id: NotSet,
        user_id: Set(dto.user_id),
        playstyle: Set(dto.playstyle),
        difficulty: Set(dto.difficulty),
        config: Set(dto.config),
        memory_level: Set(dto.memory_level),
        created_at: Set(now),
        updated_at: Set(now),
    };

    profile_active.insert(txn).await
}

pub async fn find_by_user_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
) -> Result<Option<ai_profiles::Model>, sea_orm::DbErr> {
    ai_profiles::Entity::find()
        .filter(ai_profiles::Column::UserId.eq(user_id))
        .one(conn)
        .await
}

pub async fn update_profile(
    txn: &DatabaseTransaction,
    dto: AiProfileUpdate,
) -> Result<ai_profiles::Model, sea_orm::DbErr> {
    let profile = ai_profiles::ActiveModel {
        id: Set(dto.id),
        user_id: Set(dto.user_id),
        playstyle: Set(dto.playstyle),
        difficulty: Set(dto.difficulty),
        config: Set(dto.config),
        memory_level: Set(dto.memory_level),
        created_at: NotSet,
        updated_at: Set(time::OffsetDateTime::now_utc()),
    };
    profile.update(txn).await
}
