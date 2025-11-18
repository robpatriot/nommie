//! DTOs for scores_sea adapter.

/// DTO for creating a score.
#[derive(Debug, Clone)]
pub struct ScoreCreate {
    pub round_id: i64,
    pub player_seat: u8,
    pub bid_value: u8,
    pub tricks_won: u8,
    pub bid_met: bool,
    pub base_score: u8,
    pub bonus: u8,
    pub round_score: u8,
    pub total_score_after: i16,
}
