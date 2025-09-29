use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::entities::game_players;
use crate::error::AppError;

/// Represents a user's membership in a game
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameMembership {
    pub game_id: i64,
    pub user_id: i64,
    pub membership_id: i64,
    pub turn_order: i32,
    pub is_ready: bool,
    pub role: GameRole,
}

/// Game roles for membership validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Find a user's membership in a specific game
pub async fn find_membership(
    game_id: i64,
    user_id: i64,
    conn: &impl ConnectionTrait,
) -> Result<Option<GameMembership>, AppError> {
    let membership = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(game_id))
        .filter(game_players::Column::UserId.eq(user_id))
        .one(conn)
        .await
        .map_err(|e| AppError::db(format!("Failed to query game membership: {e}")))?;

    Ok(membership.map(|m| GameMembership {
        game_id: m.game_id,
        user_id: m.user_id,
        membership_id: m.id,
        turn_order: m.turn_order,
        is_ready: m.is_ready,
        role: GameRole::Player, // For now, all members are players
    }))
}
