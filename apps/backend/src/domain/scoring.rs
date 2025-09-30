use crate::domain::rules::PLAYERS;
use crate::domain::state::{GameState, Phase};

/// Apply per-round scoring and transition to Complete.
pub fn apply_round_scoring(state: &mut GameState) {
    if state.phase != Phase::Scoring {
        return;
    }
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
}


