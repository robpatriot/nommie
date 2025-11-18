//! AI profile repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::ai_profiles_sea as ai_profiles_adapter;
use crate::entities::ai_profiles;
use crate::errors::domain::DomainError;

/// Values persisted for AI profiles
#[derive(Debug, Clone, PartialEq)]
pub struct AiProfile {
    pub id: i64,
    pub registry_name: String,
    pub registry_version: String,
    pub variant: String,
    pub display_name: String,
    pub playstyle: Option<String>,
    pub difficulty: Option<i32>,
    pub config: Option<serde_json::Value>,
    pub memory_level: Option<i32>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

/// Draft data for creating a new AI profile
#[derive(Debug, Clone, PartialEq)]
pub struct AiProfileDraft {
    pub registry_name: String,
    pub registry_version: String,
    pub variant: String,
    pub display_name: String,
    pub playstyle: Option<String>,
    pub difficulty: Option<i32>,
    pub config: Option<serde_json::Value>,
    pub memory_level: Option<i32>,
}

impl AiProfileDraft {
    pub fn new(
        registry_name: impl Into<String>,
        registry_version: impl Into<String>,
        variant: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            registry_name: registry_name.into(),
            registry_version: registry_version.into(),
            variant: variant.into(),
            display_name: display_name.into(),
            playstyle: None,
            difficulty: None,
            config: None,
            memory_level: None,
        }
    }

    pub fn with_playstyle(mut self, playstyle: impl Into<String>) -> Self {
        self.playstyle = Some(playstyle.into());
        self
    }

    pub fn with_difficulty(mut self, difficulty: i32) -> Self {
        self.difficulty = Some(difficulty);
        self
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_memory_level(mut self, memory_level: i32) -> Self {
        self.memory_level = Some(memory_level);
        self
    }
}

impl From<ai_profiles::Model> for AiProfile {
    fn from(model: ai_profiles::Model) -> Self {
        Self {
            id: model.id,
            registry_name: model.registry_name,
            registry_version: model.registry_version,
            variant: model.variant,
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

/// Create a new AI profile in the catalog.
pub async fn create_profile(
    txn: &DatabaseTransaction,
    draft: AiProfileDraft,
) -> Result<AiProfile, DomainError> {
    let mut dto = ai_profiles_adapter::AiProfileCreate::new(
        draft.registry_name,
        draft.registry_version,
        draft.variant,
        draft.display_name,
    );
    if let Some(ps) = draft.playstyle {
        dto = dto.with_playstyle(ps);
    }
    if let Some(diff) = draft.difficulty {
        dto = dto.with_difficulty(diff);
    }
    if let Some(cfg) = draft.config {
        dto = dto.with_config(cfg);
    }
    if let Some(ml) = draft.memory_level {
        dto = dto.with_memory_level(ml);
    }
    let profile = ai_profiles_adapter::create_profile(txn, dto).await?;
    Ok(AiProfile::from(profile))
}

/// Find an AI profile by ID.
pub async fn find_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    id: i64,
) -> Result<Option<AiProfile>, DomainError> {
    let profile = ai_profiles_adapter::find_by_id(conn, id).await?;
    Ok(profile.map(AiProfile::from))
}

/// Find a profile by registry signature.
pub async fn find_by_registry_variant<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    registry_name: &str,
    registry_version: &str,
    variant: &str,
) -> Result<Option<AiProfile>, DomainError> {
    let profile = ai_profiles_adapter::find_by_registry_variant(
        conn,
        registry_name,
        registry_version,
        variant,
    )
    .await?;
    Ok(profile.map(AiProfile::from))
}

pub async fn list_all<C: ConnectionTrait + Send + Sync>(
    conn: &C,
) -> Result<Vec<AiProfile>, DomainError> {
    let profiles = ai_profiles_adapter::list_all(conn).await?;
    Ok(profiles.into_iter().map(AiProfile::from).collect())
}

/// Find AI profiles for multiple IDs (batch query for optimization).
pub async fn find_batch_by_ids<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    profile_ids: &[i64],
) -> Result<Vec<ai_profiles::Model>, DomainError> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let profiles = ai_profiles::Entity::find()
        .filter(ai_profiles::Column::Id.is_in(profile_ids.iter().copied()))
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
    let mut dto = ai_profiles_adapter::AiProfileUpdate::new(profile.id);
    dto = dto
        .with_registry_name(profile.registry_name.clone())
        .with_registry_version(profile.registry_version.clone())
        .with_variant(profile.variant.clone())
        .with_display_name(profile.display_name);
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
