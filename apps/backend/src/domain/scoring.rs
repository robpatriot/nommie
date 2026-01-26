use crate::domain::rules::PLAYERS;
use crate::domain::state::{require_hand_size, GameState, Phase};

/// Result of applying round scoring, describing what state changes occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRoundScoringResult {
    /// Round score deltas for each player (scores added this round).
    pub round_score_deltas: [i16; PLAYERS],
}

/// Apply per-round scoring and transition to Complete.
///
/// Notes:
/// - If called outside `Phase::Scoring`, this is a no-op.
/// - If required invariants are missing (e.g. `hand_size` is None) or inconsistent,
///   this is a no-op (no panics, no assertions).
pub fn apply_round_scoring(state: &mut GameState) -> ApplyRoundScoringResult {
    let mut round_score_deltas = [0i16; PLAYERS];

    // Idempotence / phase guard
    if state.phase != Phase::Scoring {
        return ApplyRoundScoringResult { round_score_deltas };
    }

    // In Scoring, hand_size must be present.
    let hand_size = match require_hand_size(state, "apply_round_scoring") {
        Ok(h) => h,
        Err(_) => return ApplyRoundScoringResult { round_score_deltas },
    };

    // Invariant checks (non-panicking). If violated, no-op.
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    if tricks_sum != hand_size {
        return ApplyRoundScoringResult { round_score_deltas };
    }

    if !state.round.bids.iter().all(|bid| bid.is_some()) {
        return ApplyRoundScoringResult { round_score_deltas };
    }

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

    for pid in 0..PLAYERS {
        round_score_deltas[pid] = state.scores_total[pid] - scores_total_before[pid];
    }

    ApplyRoundScoringResult { round_score_deltas }
}
