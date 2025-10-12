//! AI user and decision-making services.

use sea_orm::DatabaseTransaction;
use tracing::debug;

use crate::errors::domain::DomainError;
use crate::repos::{ai_profiles, users as users_repo};

/// AI service for managing AI users and their decisions.
pub struct AiService;

impl AiService {
    pub fn new() -> Self {
        Self
    }

    /// Create an AI user with an AI profile.
    ///
    /// Creates both a user record (with `is_ai = true`) and an associated AI profile.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `ai_type` - Type/playstyle of AI (e.g., "random", "aggressive", "defensive")
    /// * `config` - Optional JSON config for the AI (e.g., seed, difficulty settings)
    /// * `memory_level` - Optional memory level (0-100, where 100 is perfect memory)
    ///
    /// # Returns
    /// User ID of the created AI user
    pub async fn create_ai_user(
        &self,
        txn: &DatabaseTransaction,
        ai_type: &str,
        config: Option<serde_json::Value>,
        memory_level: Option<i32>,
    ) -> Result<i64, DomainError> {
        // Generate unique sub for AI
        let random_id = rand::random::<u32>();
        let sub = format!("ai_{ai_type}_{random_id}");
        let username = format!("AI {ai_type}");

        debug!(
            ai_type = %ai_type,
            sub = %sub,
            memory_level = ?memory_level,
            "Creating AI user"
        );

        // Create user with is_ai = true
        let user = users_repo::create_user(txn, &sub, &username, true).await?;

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
            "AI user created successfully"
        );

        Ok(user.id)
    }

    // Future: Add decision-making methods
    // pub async fn get_bid_decision(&self, ...) -> Result<u8, DomainError>
    // pub async fn get_trump_decision(&self, ...) -> Result<Trump, DomainError>
    // pub async fn get_play_decision(&self, ...) -> Result<Card, DomainError>
}

impl Default for AiService {
    fn default() -> Self {
        Self::new()
    }
}
