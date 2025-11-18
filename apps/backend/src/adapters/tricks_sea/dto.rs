//! DTOs for tricks_sea adapter.

use crate::entities::round_tricks::CardSuit;

/// DTO for creating a trick.
#[derive(Debug, Clone)]
pub struct TrickCreate {
    pub round_id: i64,
    pub trick_no: u8,
    pub lead_suit: CardSuit,
    pub winner_seat: u8,
}
