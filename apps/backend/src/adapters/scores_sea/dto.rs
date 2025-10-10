//! DTOs for scores_sea adapter.

/// DTO for creating a score.
#[derive(Debug, Clone)]
pub struct ScoreCreate {
    pub round_id: i64,
    pub player_seat: i16,
    pub bid_value: i16,
    pub tricks_won: i16,
    pub bid_met: bool,
    pub base_score: i16,
    pub bonus: i16,
    pub round_score: i16,
    pub total_score_after: i16,
}
