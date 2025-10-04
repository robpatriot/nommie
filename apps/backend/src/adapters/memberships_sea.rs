//! SeaORM adapter for membership repository.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, QueryFilter, Set};

use crate::db::{DbConn, as_database_connection, as_database_transaction};
use crate::entities::game_players;
use crate::errors::domain::{DomainError, InfraErrorKind};
use crate::repos::memberships::{GameMembership, GameRole, MembershipRepo};

/// SeaORM implementation of MembershipRepo.
#[derive(Debug, Default)]
pub struct MembershipRepoSea;

impl MembershipRepoSea {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MembershipRepo for MembershipRepoSea {
    async fn find_membership(
        &self,
        conn: &dyn DbConn,
        game_id: i64,
        user_id: i64,
    ) -> Result<Option<GameMembership>, DomainError> {
        let membership = {
            if let Some(txn) = as_database_transaction(conn) {
                game_players::Entity::find()
                    .filter(game_players::Column::GameId.eq(game_id))
                    .filter(game_players::Column::UserId.eq(user_id))
                    .one(txn)
                    .await
            } else if let Some(db) = as_database_connection(conn) {
                game_players::Entity::find()
                    .filter(game_players::Column::GameId.eq(game_id))
                    .filter(game_players::Column::UserId.eq(user_id))
                    .one(db)
                    .await
            } else {
                return Err(DomainError::infra(
                    InfraErrorKind::Other("Connection type".to_string()),
                    "Unsupported DbConn type for SeaORM".to_string(),
                ));
            }
        }
            .map_err(|e| DomainError::infra(
                InfraErrorKind::Other("Database error".to_string()),
                format!("Failed to query game membership: {}", e)
            ))?;

        Ok(membership.map(|m| GameMembership {
            id: m.id,
            game_id: m.game_id,
            user_id: m.user_id,
            turn_order: m.turn_order,
            is_ready: m.is_ready,
            role: GameRole::Player, // For now, all members are players
        }))
    }

    async fn create_membership(
        &self,
        conn: &dyn DbConn,
        game_id: i64,
        user_id: i64,
        turn_order: i32,
        is_ready: bool,
        role: GameRole,
    ) -> Result<GameMembership, DomainError> {
        let now = time::OffsetDateTime::now_utc();
        let membership_active = game_players::ActiveModel {
            id: NotSet,
            game_id: Set(game_id),
            user_id: Set(user_id),
            turn_order: Set(turn_order),
            is_ready: Set(is_ready),
            created_at: Set(now),
        };

        let membership = if let Some(txn) = as_database_transaction(conn) {
            membership_active.insert(txn).await
        } else if let Some(db) = as_database_connection(conn) {
            membership_active.insert(db).await
        } else {
            return Err(DomainError::infra(
                InfraErrorKind::Other("Connection type".to_string()),
                "Unsupported DbConn type for SeaORM".to_string(),
            ));
        }
            .map_err(|e| DomainError::infra(
                InfraErrorKind::Other("Database error".to_string()),
                format!("Failed to create game membership: {}", e)
            ))?;

        Ok(GameMembership {
            id: membership.id,
            game_id: membership.game_id,
            user_id: membership.user_id,
            turn_order: membership.turn_order,
            is_ready: membership.is_ready,
            role,
        })
    }

    async fn update_membership(
        &self,
        conn: &dyn DbConn,
        membership: GameMembership,
    ) -> Result<GameMembership, DomainError> {
        let membership_active: game_players::ActiveModel = membership.into();
        // Note: game_players table doesn't have updated_at field

        let membership = if let Some(txn) = as_database_transaction(conn) {
            membership_active.update(txn).await
        } else if let Some(db) = as_database_connection(conn) {
            membership_active.update(db).await
        } else {
            return Err(DomainError::infra(
                InfraErrorKind::Other("Connection type".to_string()),
                "Unsupported DbConn type for SeaORM".to_string(),
            ));
        }
            .map_err(|e| DomainError::infra(
                InfraErrorKind::Other("Database error".to_string()),
                format!("Failed to update game membership: {}", e)
            ))?;

        Ok(GameMembership {
            id: membership.id,
            game_id: membership.game_id,
            user_id: membership.user_id,
            turn_order: membership.turn_order,
            is_ready: membership.is_ready,
            role: GameRole::Player, // For now, all members are players
        })
    }
}

// Conversion from SeaORM model to domain model
impl From<game_players::Model> for GameMembership {
    fn from(model: game_players::Model) -> Self {
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

impl From<GameMembership> for game_players::ActiveModel {
    fn from(domain: GameMembership) -> Self {
        Self {
            id: Set(domain.id),
            game_id: Set(domain.game_id),
            user_id: Set(domain.user_id),
            turn_order: Set(domain.turn_order),
            is_ready: Set(domain.is_ready),
            created_at: Set(time::OffsetDateTime::now_utc()),
        }
    }
}
