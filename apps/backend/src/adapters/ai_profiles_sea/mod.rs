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
        registry_name: Set(dto.registry_name),
        registry_version: Set(dto.registry_version),
        variant: Set(dto.variant),
        display_name: Set(dto.display_name),
        playstyle: Set(dto.playstyle),
        difficulty: Set(dto.difficulty),
        config: Set(dto.config),
        memory_level: Set(dto.memory_level),
        created_at: Set(now),
        updated_at: Set(now),
    };

    profile_active.insert(txn).await
}

pub async fn find_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    id: i64,
) -> Result<Option<ai_profiles::Model>, sea_orm::DbErr> {
    ai_profiles::Entity::find()
        .filter(ai_profiles::Column::Id.eq(id))
        .one(conn)
        .await
}

pub async fn find_by_registry_variant<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    registry_name: &str,
    registry_version: &str,
    variant: &str,
) -> Result<Option<ai_profiles::Model>, sea_orm::DbErr> {
    ai_profiles::Entity::find()
        .filter(ai_profiles::Column::RegistryName.eq(registry_name))
        .filter(ai_profiles::Column::RegistryVersion.eq(registry_version))
        .filter(ai_profiles::Column::Variant.eq(variant))
        .one(conn)
        .await
}

pub async fn list_all<C: ConnectionTrait + Send + Sync>(
    conn: &C,
) -> Result<Vec<ai_profiles::Model>, sea_orm::DbErr> {
    ai_profiles::Entity::find().all(conn).await
}

pub async fn update_profile(
    txn: &DatabaseTransaction,
    dto: AiProfileUpdate,
) -> Result<ai_profiles::Model, sea_orm::DbErr> {
    let mut profile = ai_profiles::ActiveModel {
        id: Set(dto.id),
        registry_name: NotSet,
        registry_version: NotSet,
        variant: NotSet,
        display_name: NotSet,
        playstyle: Set(dto.playstyle),
        difficulty: Set(dto.difficulty),
        config: Set(dto.config),
        memory_level: Set(dto.memory_level),
        created_at: NotSet,
        updated_at: Set(time::OffsetDateTime::now_utc()),
    };

    if let Some(registry_name) = dto.registry_name {
        profile.registry_name = Set(registry_name);
    }
    if let Some(registry_version) = dto.registry_version {
        profile.registry_version = Set(registry_version);
    }
    if let Some(variant) = dto.variant {
        profile.variant = Set(variant);
    }
    if let Some(display_name) = dto.display_name {
        profile.display_name = Set(display_name);
    }

    profile.update(txn).await
}
