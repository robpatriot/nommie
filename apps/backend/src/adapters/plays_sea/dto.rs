//! DTOs for plays_sea adapter.

use crate::repos::plays::Card;

/// DTO for creating a play.
#[derive(Debug, Clone)]
pub struct PlayCreate {
    pub trick_id: i64,
    pub player_seat: i16,
    pub card: Card,
    pub play_order: i16,
}
