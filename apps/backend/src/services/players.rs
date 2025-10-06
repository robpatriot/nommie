//! Player domain service.

use sea_orm::ConnectionTrait;

use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::players;

/// Player domain service.
pub struct PlayerService;

impl PlayerService {
    pub fn new() -> Self {
        Self
    }

    /// Get the display name of a player in a game by seat.
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `game_id` - The game identifier
    /// * `seat` - The seat number (0-3)
    ///
    /// # Returns
    /// * `Ok(String)` - The player's display name
    /// * `Err(DomainError)` - If seat is invalid, player not found, or DB error
    pub async fn get_display_name_by_seat<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
        seat: u8,
    ) -> Result<String, DomainError> {
        // Validate seat range
        if seat > 3 {
            return Err(DomainError::validation(
                ValidationKind::InvalidSeat,
                "Seat must be between 0 and 3",
            ));
        }

        // Call repository and map DomainError to AppError
        let display_name = players::get_display_name_by_seat(conn, game_id, seat).await?;
        Ok(display_name)
    }
}

impl Default for PlayerService {
    fn default() -> Self {
        Self::new()
    }
}
