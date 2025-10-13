//! AI user and decision-making services.

use sea_orm::DatabaseTransaction;
use tracing::debug;

use crate::errors::domain::DomainError;
use crate::repos::{ai_overrides, ai_profiles, memberships, users as users_repo};

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
pub struct AiService;

/// Optional overrides for AI instances in specific games.
#[derive(Debug, Clone, Default)]
pub struct AiInstanceOverrides {
    pub name: Option<String>,
    pub memory_level: Option<i32>,
    pub config: Option<serde_json::Value>,
}

impl AiService {
    pub fn new() -> Self {
        Self
    }

    /// Create a reusable AI template user.
    ///
    /// Creates both a user record (with `is_ai = true`) and an associated AI profile.
    /// This AI user can be reused across many games.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `name` - Display name for the AI (e.g., "Random Bot Easy", "Aggressive Alice")
    /// * `ai_type` - Type/playstyle of AI (e.g., "random", "aggressive", "defensive")
    /// * `config` - Optional JSON config for the AI (e.g., seed, difficulty settings)
    /// * `memory_level` - Optional memory level (0-100, where 100 is perfect memory)
    ///
    /// # Returns
    /// User ID of the created AI template user
    pub async fn create_ai_template_user(
        &self,
        txn: &DatabaseTransaction,
        name: impl Into<String>,
        ai_type: &str,
        config: Option<serde_json::Value>,
        memory_level: Option<i32>,
    ) -> Result<i64, DomainError> {
        let name = name.into();
        let sub = format!("ai:{}:{}", ai_type, uuid::Uuid::new_v4());

        debug!(
            ai_type = %ai_type,
            name = %name,
            memory_level = ?memory_level,
            "Creating AI template user"
        );

        // Create user with is_ai = true
        let user = users_repo::create_user(txn, &sub, &name, true).await?;

        // Create AI profile
        ai_profiles::create_profile(
            txn,
            user.id,
            Some(ai_type.to_string()),
            None,
            config,
            memory_level,
        )
        .await?;

        debug!(
            user_id = user.id,
            ai_type = %ai_type,
            name = %name,
            "AI template user created successfully"
        );

        Ok(user.id)
    }

    /// Add an AI to a game with optional per-instance overrides.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `game_id` - Game to add the AI to
    /// * `ai_user_id` - AI template user ID (from create_ai_template_user)
    /// * `seat` - Seat position (0-3)
    /// * `overrides` - Optional overrides for this specific instance
    ///
    /// # Returns
    /// Game player ID
    pub async fn add_ai_to_game(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        ai_user_id: i64,
        seat: i32,
        overrides: Option<AiInstanceOverrides>,
    ) -> Result<i64, DomainError> {
        // Create game membership
        let membership = memberships::create_membership(
            txn,
            game_id,
            ai_user_id,
            seat,
            false,
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

impl Default for AiService {
    fn default() -> Self {
        Self::new()
    }
}
