//! AI profile repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::ai_profiles_sea as ai_profiles_adapter;
use crate::entities::ai_profiles;
use crate::errors::domain::DomainError;

/// AI profile domain model
#[derive(Debug, Clone, PartialEq)]
pub struct AiProfile {
    pub id: i64,
    pub user_id: i64,
    pub display_name: String,
    pub playstyle: Option<String>,
    pub difficulty: Option<i32>,
    pub config: Option<serde_json::Value>,
    pub memory_level: Option<i32>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

impl From<ai_profiles::Model> for AiProfile {
    fn from(model: ai_profiles::Model) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            display_name: model.display_name,
            playstyle: model.playstyle,
            difficulty: model.difficulty,
            config: model.config,
            memory_level: model.memory_level,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

/// Create a new AI profile for a user.
pub async fn create_profile(
    txn: &DatabaseTransaction,
    user_id: i64,
    display_name: impl Into<String>,
    playstyle: Option<String>,
    difficulty: Option<i32>,
    config: Option<serde_json::Value>,
    memory_level: Option<i32>,
) -> Result<AiProfile, DomainError> {
    let mut dto = ai_profiles_adapter::AiProfileCreate::new(user_id, display_name);
    if let Some(ps) = playstyle {
        dto = dto.with_playstyle(ps);
    }
    if let Some(diff) = difficulty {
        dto = dto.with_difficulty(diff);
    }
    if let Some(cfg) = config {
        dto = dto.with_config(cfg);
    }
    if let Some(ml) = memory_level {
        dto = dto.with_memory_level(ml);
    }
    let profile = ai_profiles_adapter::create_profile(txn, dto).await?;
    Ok(AiProfile::from(profile))
}

/// Find an AI profile by user ID.
pub async fn find_by_user_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
) -> Result<Option<AiProfile>, DomainError> {
    let profile = ai_profiles_adapter::find_by_user_id(conn, user_id).await?;
    Ok(profile.map(AiProfile::from))
}

/// Find AI profiles for multiple user IDs (batch query for optimization).
pub async fn find_batch_by_user_ids<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_ids: &[i64],
) -> Result<Vec<ai_profiles::Model>, DomainError> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let profiles = ai_profiles::Entity::find()
        .filter(ai_profiles::Column::UserId.is_in(user_ids.iter().copied()))
        .all(conn)
        .await
        .map_err(crate::infra::db_errors::map_db_err)?;

    Ok(profiles)
}

/// Update an AI profile's configuration.
pub async fn update_profile(
    txn: &DatabaseTransaction,
    profile: AiProfile,
) -> Result<AiProfile, DomainError> {
    let mut dto = ai_profiles_adapter::AiProfileUpdate::new(profile.id, profile.user_id);
    dto = dto.with_display_name(profile.display_name);
    if let Some(ps) = profile.playstyle {
        dto = dto.with_playstyle(ps);
    }
    if let Some(diff) = profile.difficulty {
        dto = dto.with_difficulty(diff);
    }
    if let Some(cfg) = profile.config {
        dto = dto.with_config(cfg);
    }
    if let Some(ml) = profile.memory_level {
        dto = dto.with_memory_level(ml);
    }
    let updated = ai_profiles_adapter::update_profile(txn, dto).await?;
    Ok(AiProfile::from(updated))
}
