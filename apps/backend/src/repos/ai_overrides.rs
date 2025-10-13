//! AI overrides repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::ai_overrides_sea as ai_overrides_adapter;
use crate::entities::ai_overrides;
use crate::errors::domain::DomainError;

/// AI override domain model
#[derive(Debug, Clone, PartialEq)]
pub struct AiOverride {
    pub id: i64,
    pub game_player_id: i64,
    pub name: Option<String>,
    pub memory_level: Option<i32>,
    pub config: Option<serde_json::Value>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

impl From<ai_overrides::Model> for AiOverride {
    fn from(model: ai_overrides::Model) -> Self {
        Self {
            id: model.id,
            game_player_id: model.game_player_id,
            name: model.name,
            memory_level: model.memory_level,
            config: model.config,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

/// Create AI overrides for a game player.
pub async fn create_override(
    txn: &DatabaseTransaction,
    game_player_id: i64,
    name: Option<String>,
    memory_level: Option<i32>,
    config: Option<serde_json::Value>,
) -> Result<AiOverride, DomainError> {
    let mut dto = ai_overrides_adapter::AiOverrideCreate::new(game_player_id);
    if let Some(n) = name {
        dto = dto.with_name(n);
    }
    if let Some(ml) = memory_level {
        dto = dto.with_memory_level(ml);
    }
    if let Some(cfg) = config {
        dto = dto.with_config(cfg);
    }
    let override_model = ai_overrides_adapter::create_override(txn, dto).await?;
    Ok(AiOverride::from(override_model))
}

/// Find AI overrides by game_player_id.
pub async fn find_by_game_player_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_player_id: i64,
) -> Result<Option<AiOverride>, DomainError> {
    let override_model = ai_overrides_adapter::find_by_game_player_id(conn, game_player_id).await?;
    Ok(override_model.map(AiOverride::from))
}

/// Update AI overrides.
pub async fn update_override(
    txn: &DatabaseTransaction,
    override_data: AiOverride,
) -> Result<AiOverride, DomainError> {
    let mut dto =
        ai_overrides_adapter::AiOverrideUpdate::new(override_data.id, override_data.game_player_id);
    if let Some(n) = override_data.name {
        dto = dto.with_name(n);
    }
    if let Some(ml) = override_data.memory_level {
        dto = dto.with_memory_level(ml);
    }
    if let Some(cfg) = override_data.config {
        dto = dto.with_config(cfg);
    }
    let updated = ai_overrides_adapter::update_override(txn, dto).await?;
    Ok(AiOverride::from(updated))
}

/// Delete AI overrides by game_player_id.
pub async fn delete_by_game_player_id(
    txn: &DatabaseTransaction,
    game_player_id: i64,
) -> Result<(), DomainError> {
    ai_overrides_adapter::delete_by_game_player_id(txn, game_player_id).await?;
    Ok(())
}
