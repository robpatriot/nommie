//! DTOs for hands_sea adapter.

use crate::repos::hands::Card;

/// DTO for creating a hand.
#[derive(Debug, Clone)]
pub struct HandCreate {
    pub round_id: i64,
    pub player_seat: u8,
    pub cards: Vec<Card>,
}
