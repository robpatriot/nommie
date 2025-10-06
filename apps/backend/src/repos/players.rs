//! Player repository functions for domain layer (generic over ConnectionTrait).

use sea_orm::ConnectionTrait;

use crate::adapters::players_sea as players_adapter;
use crate::errors::domain::{DomainError, NotFoundKind};

// Free functions (generic) mirroring the previous trait methods

/// Get the display name of a player in a game by seat.
///
/// # Arguments
/// * `conn` - Database connection
/// * `game_id` - The game identifier
/// * `seat` - The seat number (0-3)
///
/// # Returns
/// * `Ok(String)` - The player's display name
/// * `Err(DomainError)` - If player not found or DB error
pub async fn get_display_name_by_seat<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
    seat: u8,
) -> Result<String, DomainError> {
    let game_player = players_adapter::get_display_name_by_seat(conn, game_id, seat).await?;

    match game_player {
        Some((_game_player, user)) => {
            // Use username if available, otherwise fall back to sub
            let display_name = user.username.unwrap_or_else(|| user.sub.clone());
            Ok(display_name)
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
