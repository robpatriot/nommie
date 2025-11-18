//! DTOs for rounds_sea adapter.

use crate::entities::game_rounds::CardTrump;

/// DTO for creating a new round.
#[derive(Debug, Clone)]
pub struct RoundCreate {
    pub game_id: i64,
    pub round_no: u8,
    pub hand_size: u8,
    pub dealer_pos: u8,
}

/// DTO for updating trump selection.
#[derive(Debug, Clone)]
pub struct RoundUpdateTrump {
    pub round_id: i64,
    pub trump: CardTrump,
}
