//! SeaORM adapter for player repository.

use async_trait::async_trait;
use sea_orm::{ConnectionTrait, EntityTrait, QueryFilter, ColumnTrait};

use crate::errors::domain::DomainError;
use crate::entities::{game_players, users};
use crate::repos::players::PlayerRepo;

/// SeaORM implementation of PlayerRepo.
pub struct PlayerRepoSea<'a, C: ConnectionTrait + Send + Sync> {
    conn: &'a C,
}

impl<'a, C: ConnectionTrait + Send + Sync> PlayerRepoSea<'a, C> {
    pub fn new(conn: &'a C) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl<'a, C: ConnectionTrait + Send + Sync> PlayerRepo for PlayerRepoSea<'a, C> {
    async fn get_display_name_by_seat(
        &self,
        game_id: i64,
        seat: u8,
    ) -> Result<String, DomainError> {
        // Find game_players record by game_id and turn_order (which maps to seat)
        let game_player = game_players::Entity::find()
            .filter(game_players::Column::GameId.eq(game_id))
            .filter(game_players::Column::TurnOrder.eq(seat as i32))
            .find_also_related(users::Entity)
            .one(self.conn)
            .await
            .map_err(|e| DomainError::infra(
                crate::errors::domain::InfraErrorKind::Other("Database error".to_string()),
                format!("Database error: {}", e)
            ))?;

        match game_player {
            Some((_game_player, Some(user))) => {
                // Use username if available, otherwise fall back to sub
                let display_name = user.username
                    .unwrap_or_else(|| user.sub.clone());
                Ok(display_name)
            }
            Some((_game_player, None)) => {
                // Game player exists but user is missing (data corruption)
                Err(DomainError::infra(
                    crate::errors::domain::InfraErrorKind::DataCorruption,
                    "User not found for game player"
                ))
            }
            None => {
                // No game player found for this seat
                Err(DomainError::not_found(
                    crate::errors::domain::NotFoundKind::Other("Player".to_string()),
                    "Player not found at seat"
                ))
            }
        }
    }
}
