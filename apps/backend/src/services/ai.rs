//! AI user and decision-making services.

use sea_orm::DatabaseTransaction;
use tracing::debug;

use crate::errors::domain::DomainError;
use crate::repos::{ai_overrides, memberships};

/// Merge two JSON configs, with override taking precedence.
pub fn merge_json_configs(
    base: Option<&serde_json::Value>,
    override_config: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    match (base, override_config) {
        (None, None) => None,
        (Some(b), None) => Some(b.clone()),
        (None, Some(o)) => Some(o.clone()),
        (Some(base_val), Some(override_val)) => {
            // If both are objects, merge them; otherwise override wins
            if let (Some(base_obj), Some(override_obj)) =
                (base_val.as_object(), override_val.as_object())
            {
                let mut merged = base_obj.clone();
                for (key, value) in override_obj {
                    merged.insert(key.clone(), value.clone());
                }
                Some(serde_json::Value::Object(merged))
            } else {
                Some(override_val.clone())
            }
        }
    }
}

/// AI service for managing AI users and their decisions.
#[derive(Default)]
pub struct AiService;

/// Optional overrides for AI instances in specific games.
#[derive(Debug, Clone, Default)]
pub struct AiInstanceOverrides {
    pub name: Option<String>,
    pub memory_level: Option<i32>,
    pub config: Option<serde_json::Value>,
}

impl AiService {
    /// Add an AI to a game with optional per-instance overrides.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `game_id` - Game to add the AI to
    /// * `ai_profile_id` - Catalog AI profile ID
    /// * `seat` - Seat position (0-3)
    /// * `overrides` - Optional overrides for this specific instance
    ///
    /// # Returns
    /// Game player ID
    pub async fn add_ai_to_game(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        ai_profile_id: i64,
        seat: i32,
        overrides: Option<AiInstanceOverrides>,
    ) -> Result<i64, DomainError> {
        // Create game membership
        let membership = memberships::create_ai_membership(
            txn,
            game_id,
            ai_profile_id,
            seat,
            true,
            memberships::GameRole::Player,
        )
        .await?;

        // If overrides provided, create override record
        if let Some(ovr) = overrides {
            if ovr.name.is_some() || ovr.memory_level.is_some() || ovr.config.is_some() {
                ai_overrides::create_override(
                    txn,
                    membership.id,
                    ovr.name,
                    ovr.memory_level,
                    ovr.config,
                )
                .await?;

                debug!(
                    game_player_id = membership.id,
                    "Created AI overrides for instance"
                );
            }
        }

        Ok(membership.id)
    }
}
