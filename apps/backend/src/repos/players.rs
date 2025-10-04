//! Player repository trait for domain layer.

use async_trait::async_trait;

use crate::db::DbConn;
use crate::errors::domain::DomainError;

/// Repository trait for player operations.
/// 
/// This trait is domain-facing and contains no SeaORM imports.
/// Adapters implement this trait using SeaORM entities.
#[async_trait]
pub trait PlayerRepo: Send + Sync {
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
    async fn get_display_name_by_seat(
        &self,
        conn: &dyn DbConn,
        game_id: i64,
        seat: u8,
    ) -> Result<String, DomainError>;
}
