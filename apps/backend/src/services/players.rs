//! Player domain service.

use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::repos::players::PlayerRepo;

/// Player domain service.
pub struct PlayerService<R: PlayerRepo> {
    repo: R,
}

impl<R: PlayerRepo> PlayerService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Get the display name of a player in a game by seat.
    /// 
    /// # Arguments
    /// * `game_id` - The game identifier
    /// * `seat` - The seat number (0-3)
    /// 
    /// # Returns
    /// * `Ok(String)` - The player's display name
    /// * `Err(AppError)` - If seat is invalid, player not found, or DB error
    pub async fn get_display_name_by_seat(
        &self,
        game_id: i64,
        seat: u8,
    ) -> Result<String, AppError> {
        // Validate seat range
        if seat > 3 {
            return Err(AppError::Validation {
                code: ErrorCode::InvalidSeat,
                detail: "Seat must be between 0 and 3".to_string(),
                status: actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
            });
        }

        // Call repository and map DomainError to AppError
        let display_name = self.repo.get_display_name_by_seat(game_id, seat).await?;
        Ok(display_name)
    }
}
