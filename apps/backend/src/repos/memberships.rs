//! Membership repository functions for domain layer (generic over ConnectionTrait).

use sea_orm::ConnectionTrait;

use crate::adapters::memberships_sea as memberships_adapter;
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

// Free functions (generic) mirroring the previous trait methods

pub async fn find_membership<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
    user_id: i64,
) -> Result<Option<GameMembership>, DomainError> {
    let membership = memberships_adapter::find_membership(conn, game_id, user_id).await?;
    Ok(membership.map(GameMembership::from))
}

pub async fn create_membership<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
    user_id: i64,
    turn_order: i32,
    is_ready: bool,
    role: GameRole,
) -> Result<GameMembership, DomainError> {
    let dto = memberships_adapter::MembershipCreate::new(game_id, user_id, turn_order, is_ready);
    let membership = memberships_adapter::create_membership(conn, dto).await?;
    Ok(GameMembership {
        id: membership.id,
        game_id: membership.game_id,
        user_id: membership.user_id,
        turn_order: membership.turn_order,
        is_ready: membership.is_ready,
        role,
    })
}

pub async fn update_membership<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    membership: GameMembership,
) -> Result<GameMembership, DomainError> {
    let updated = memberships_adapter::update_membership(
        conn,
        membership.id,
        membership.game_id,
        membership.user_id,
        membership.turn_order,
        membership.is_ready,
    )
    .await?;
    Ok(GameMembership {
        id: updated.id,
        game_id: updated.game_id,
        user_id: updated.user_id,
        turn_order: updated.turn_order,
        is_ready: updated.is_ready,
        role: GameRole::Player, // For now, all members are players
    })
}

// Conversions between SeaORM models and domain models

impl From<crate::entities::game_players::Model> for GameMembership {
    fn from(model: crate::entities::game_players::Model) -> Self {
        Self {
            id: model.id,
            game_id: model.game_id,
            user_id: model.user_id,
            turn_order: model.turn_order,
            is_ready: model.is_ready,
            role: GameRole::Player, // For now, all members are players
        }
    }
}
