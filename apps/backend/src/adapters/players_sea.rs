//! SeaORM adapter for player repository.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::db::{as_database_connection, as_database_transaction, DbConn};
use crate::entities::{game_players, users};
use crate::errors::domain::{DomainError, InfraErrorKind, NotFoundKind};
use crate::repos::players::PlayerRepo;

/// SeaORM implementation of PlayerRepo.
#[derive(Debug, Default)]
pub struct PlayerRepoSea;

impl PlayerRepoSea {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PlayerRepo for PlayerRepoSea {
    async fn get_display_name_by_seat(
        &self,
        conn: &dyn DbConn,
        game_id: i64,
        seat: u8,
    ) -> Result<String, DomainError> {
        // Find game_players record by game_id and turn_order (which maps to seat)
        let game_player = {
            if let Some(txn) = as_database_transaction(conn) {
                game_players::Entity::find()
                    .filter(game_players::Column::GameId.eq(game_id))
                    .filter(game_players::Column::TurnOrder.eq(seat as i32))
                    .find_also_related(users::Entity)
                    .one(txn)
                    .await
            } else if let Some(db) = as_database_connection(conn) {
                game_players::Entity::find()
                    .filter(game_players::Column::GameId.eq(game_id))
                    .filter(game_players::Column::TurnOrder.eq(seat as i32))
                    .find_also_related(users::Entity)
                    .one(db)
                    .await
            } else {
                return Err(DomainError::infra(
                    InfraErrorKind::Other("Connection type".to_string()),
                    "Unsupported DbConn type for SeaORM".to_string(),
                ));
            }
        }
        .map_err(|e| {
            DomainError::infra(
                InfraErrorKind::Other("Database error".to_string()),
                format!("Database error: {e}"),
            )
        })?;

        match game_player {
            Some((_game_player, Some(user))) => {
                // Use username if available, otherwise fall back to sub
                let display_name = user.username.unwrap_or_else(|| user.sub.clone());
                Ok(display_name)
            }
            Some((_game_player, None)) => {
                // Game player exists but user is missing (data corruption)
                Err(DomainError::infra(
                    InfraErrorKind::DataCorruption,
                    "User not found for game player",
                ))
            }
            None => {
                // No game player found for this seat
                Err(DomainError::not_found(
                    NotFoundKind::Player,
                    "Player not found at seat",
                ))
            }
        }
    }
}
