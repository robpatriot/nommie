use crate::domain::rules::PLAYERS;
use crate::domain::state::{GameState, Phase};

/// Result of applying round scoring, describing what state changes occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRoundScoringResult {
    /// Round score deltas for each player (scores added this round).
    pub round_score_deltas: [i16; PLAYERS],
}

/// Apply per-round scoring and transition to Complete.
pub fn apply_round_scoring(state: &mut GameState) -> ApplyRoundScoringResult {
    let mut round_score_deltas = [0i16; PLAYERS];

    // Idempotence: if scoring has already been applied (or we're in any non-Scoring phase),
    // this call is a no-op and returns zero deltas.
    if state.phase != Phase::Scoring {
        return ApplyRoundScoringResult { round_score_deltas };
    }

    // Debug assertions for core invariants
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    debug_assert_eq!(
        tricks_sum, state.hand_size,
        "Sum of tricks won ({}) must equal hand size ({})",
        tricks_sum, state.hand_size
    );

    debug_assert!(
        state.round.bids.iter().all(|bid| bid.is_some()),
        "All players must have placed bids before scoring"
    );

    let scores_total_before = state.scores_total;

    for pid in 0..PLAYERS {
        let tricks = state.round.tricks_won[pid] as i16;
        let bid_opt = state.round.bids[pid];
        let bonus = match bid_opt {
            Some(b) if b as i16 == tricks => 10,
            _ => 0,
        };
        state.scores_total[pid] += tricks + bonus;
    }
    state.phase = Phase::Complete;

    // Calculate deltas
    for pid in 0..PLAYERS {
        round_score_deltas[pid] = state.scores_total[pid] - scores_total_before[pid];
    }

    ApplyRoundScoringResult { round_score_deltas }
}
