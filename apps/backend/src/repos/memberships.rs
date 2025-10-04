//! Membership repository trait for domain layer.

use async_trait::async_trait;

use crate::db::DbConn;
use crate::errors::domain::DomainError;

/// Game membership domain model
#[derive(Debug, Clone, PartialEq)]
pub struct GameMembership {
    pub id: i64,
    pub game_id: i64,
    pub user_id: i64,
    pub turn_order: i32,
    pub is_ready: bool,
    pub role: GameRole,
}

/// Game roles for membership validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameRole {
    /// Regular player in the game
    Player,
    /// Spectator (can view but not participate)
    Spectator,
}

impl GameRole {
    /// Check if this role has at least the required level
    pub fn has_at_least(&self, required: GameRole) -> bool {
        match (self, required) {
            (GameRole::Player, GameRole::Player) => true,
            (GameRole::Player, GameRole::Spectator) => true,
            (GameRole::Spectator, GameRole::Player) => false,
            (GameRole::Spectator, GameRole::Spectator) => true,
        }
    }
}

/// Repository trait for membership operations.
/// 
/// This trait is domain-facing and contains no SeaORM imports.
/// Adapters implement this trait using SeaORM entities.
#[async_trait]
pub trait MembershipRepo: Send + Sync {
    /// Find a user's membership in a specific game
    async fn find_membership(
        &self,
        conn: &dyn DbConn,
        game_id: i64,
        user_id: i64,
    ) -> Result<Option<GameMembership>, DomainError>;

    /// Create a new membership
    async fn create_membership(
        &self,
        conn: &dyn DbConn,
        game_id: i64,
        user_id: i64,
        turn_order: i32,
        is_ready: bool,
        role: GameRole,
    ) -> Result<GameMembership, DomainError>;

    /// Update membership
    async fn update_membership(
        &self,
        conn: &dyn DbConn,
        membership: GameMembership,
    ) -> Result<GameMembership, DomainError>;
}
