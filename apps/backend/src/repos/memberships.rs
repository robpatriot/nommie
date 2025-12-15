//! Membership repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::memberships_sea as memberships_adapter;
use crate::entities::game_players::PlayerKind;
use crate::errors::domain::DomainError;

/// Game membership domain model
#[derive(Debug, Clone, PartialEq)]
pub struct GameMembership {
    pub id: i64,
    pub game_id: i64,
    pub player_kind: PlayerKind,
    pub user_id: Option<i64>,
    pub ai_profile_id: Option<i64>,
    pub turn_order: u8,
    pub is_ready: bool,
    pub role: GameRole,
}

/// Game roles for membership validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameRole {
    /// Regular player in the game
    Player,
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

pub async fn create_membership(
    txn: &DatabaseTransaction,
    game_id: i64,
    user_id: i64,
    turn_order: u8,
    is_ready: bool,
    role: GameRole,
) -> Result<GameMembership, DomainError> {
    let dto = memberships_adapter::MembershipCreate {
        game_id,
        player_kind: PlayerKind::Human,
        human_user_id: Some(user_id),
        ai_profile_id: None,
        turn_order,
        is_ready,
    };
    let membership = memberships_adapter::create_membership(txn, dto).await?;

    Ok(GameMembership {
        id: membership.id,
        game_id: membership.game_id,
        player_kind: PlayerKind::Human,
        user_id: membership.human_user_id,
        ai_profile_id: membership.ai_profile_id,
        turn_order: membership.turn_order as u8,
        is_ready: membership.is_ready,
        role,
    })
}

pub async fn create_ai_membership(
    txn: &DatabaseTransaction,
    game_id: i64,
    ai_profile_id: i64,
    turn_order: u8,
    is_ready: bool,
    role: GameRole,
) -> Result<GameMembership, DomainError> {
    let dto = memberships_adapter::MembershipCreate {
        game_id,
        player_kind: PlayerKind::Ai,
        human_user_id: None,
        ai_profile_id: Some(ai_profile_id),
        turn_order,
        is_ready,
    };
    let membership = memberships_adapter::create_membership(txn, dto).await?;

    Ok(GameMembership {
        id: membership.id,
        game_id: membership.game_id,
        player_kind: PlayerKind::Ai,
        user_id: membership.human_user_id,
        ai_profile_id: membership.ai_profile_id,
        turn_order: membership.turn_order as u8,
        is_ready: membership.is_ready,
        role,
    })
}

pub async fn update_membership(
    txn: &DatabaseTransaction,
    membership: GameMembership,
) -> Result<GameMembership, DomainError> {
    let dto = memberships_adapter::MembershipUpdate {
        id: membership.id,
        game_id: membership.game_id,
        player_kind: membership.player_kind.clone(),
        human_user_id: membership.user_id,
        ai_profile_id: membership.ai_profile_id,
        turn_order: membership.turn_order,
        is_ready: membership.is_ready,
    };
    let updated = memberships_adapter::update_membership(txn, dto).await?;
    Ok(GameMembership {
        id: updated.id,
        game_id: updated.game_id,
        player_kind: updated.player_kind,
        user_id: updated.human_user_id,
        ai_profile_id: updated.ai_profile_id,
        turn_order: updated.turn_order as u8,
        is_ready: updated.is_ready,
        role: GameRole::Player, // For now, all members are players
    })
}

// Conversions between SeaORM models and domain models

pub async fn find_all_by_game<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
) -> Result<Vec<GameMembership>, DomainError> {
    let memberships = memberships_adapter::find_all_by_game(conn, game_id).await?;
    Ok(memberships.into_iter().map(GameMembership::from).collect())
}

pub async fn delete_membership(
    txn: &DatabaseTransaction,
    membership_id: i64,
) -> Result<(), DomainError> {
    memberships_adapter::delete_membership(txn, membership_id).await?;
    Ok(())
}

/// Set the ready status of a membership.
pub async fn set_membership_ready(
    txn: &DatabaseTransaction,
    membership_id: i64,
    is_ready: bool,
) -> Result<GameMembership, DomainError> {
    let dto = memberships_adapter::MembershipSetReady::new(membership_id, is_ready);
    let updated = memberships_adapter::set_membership_ready(txn, dto).await?;

    Ok(GameMembership {
        id: updated.id,
        game_id: updated.game_id,
        player_kind: updated.player_kind,
        user_id: updated.human_user_id,
        ai_profile_id: updated.ai_profile_id,
        turn_order: updated.turn_order as u8,
        is_ready: updated.is_ready,
        role: GameRole::Player, // For now, all members are players
    })
}

impl From<crate::entities::game_players::Model> for GameMembership {
    fn from(model: crate::entities::game_players::Model) -> Self {
        Self {
            id: model.id,
            game_id: model.game_id,
            player_kind: model.player_kind,
            user_id: model.human_user_id,
            ai_profile_id: model.ai_profile_id,
            turn_order: model.turn_order as u8,
            is_ready: model.is_ready,
            role: GameRole::Player, // For now, all members are players
        }
    }
}
