// apps/backend/src/domain/game_transition.rs

use crate::domain::state::PlayerId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameTransition {
    /// Edge-triggered: the turn became a specific player.
    TurnBecame { player_id: PlayerId },
}

/// Derive domain transitions from before/after turn state.
///
/// Rules:
/// - Edge-triggered only (emit only on change).
/// - Only emit when `after_turn` is `Some(_)`.
/// - Ignore transitions to `None` for now (no actionable "your turn" event).
pub fn derive_game_transitions(
    before_turn: Option<PlayerId>,
    after_turn: Option<PlayerId>,
) -> Vec<GameTransition> {
    match (before_turn, after_turn) {
        // Edge-triggered: turn became Some(player)
        (_, Some(player_id)) if before_turn != Some(player_id) => {
            vec![GameTransition::TurnBecame { player_id }]
        }

        // All other cases: no transition
        _ => Vec::new(),
    }
}
