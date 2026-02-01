// apps/backend/src/domain/game_transition.rs

use crate::domain::state::PlayerId;
use crate::entities::games::GameState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameLifecycleView {
    pub version: i32,
    pub turn: Option<PlayerId>,
    pub state: GameState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameTransition {
    /// Edge-triggered: the turn became a specific player.
    TurnBecame { player_id: PlayerId },

    /// Edge-triggered: Game moved from Lobby -> Active (Bidding/etc)
    GameStarted,

    /// Edge-triggered: Game moved from Active -> Completed
    GameEnded,

    /// Explicit: User joined the game
    PlayerJoined { user_id: i64 },

    /// Explicit: User left the game
    PlayerLeft { user_id: i64 },

    /// Explicit: User rejoined the game
    PlayerRejoined { user_id: i64 },

    /// Edge-triggered: Game moved from Active/Lobby -> Abandoned
    GameAbandoned,
}

/// Derive domain transitions from before/after lifecycle state.
pub fn derive_game_transitions(
    before: &GameLifecycleView,
    after: &GameLifecycleView,
) -> Vec<GameTransition> {
    let mut transitions = Vec::new();

    // 1. Turn change
    if let (_, Some(player_id)) = (before.turn, after.turn) {
        if before.turn != Some(player_id) {
            transitions.push(GameTransition::TurnBecame { player_id });
        }
    }

    // 2. Game Start (Lobby -> !Lobby)
    // Note: If jumping straight to Abandoned/Completed from Lobby, we might not count it as "Started"
    // but rather Ended/Abandoned directly.
    if before.state == GameState::Lobby
        && after.state != GameState::Lobby
        && after.state != GameState::Abandoned
        && after.state != GameState::Completed
    {
        transitions.push(GameTransition::GameStarted);
    }

    // 3. Game End (!Completed -> Completed)
    if before.state != GameState::Completed && after.state == GameState::Completed {
        transitions.push(GameTransition::GameEnded);
    }

    // 4. Game Abandoned (!Abandoned -> Abandoned)
    if before.state != GameState::Abandoned && after.state == GameState::Abandoned {
        transitions.push(GameTransition::GameAbandoned);
    }

    transitions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::games::GameState;

    fn view(state: GameState, turn: Option<u8>) -> GameLifecycleView {
        GameLifecycleView {
            version: 1,
            state,
            turn,
        }
    }

    #[test]
    fn test_derive_game_started() {
        let before = view(GameState::Lobby, None);
        let after = view(GameState::Bidding, None);
        let transitions = derive_game_transitions(&before, &after);
        assert!(transitions.contains(&GameTransition::GameStarted));
    }

    #[test]
    fn test_derive_game_ended() {
        let before = view(GameState::TrickPlay, None);
        let after = view(GameState::Completed, None);
        let transitions = derive_game_transitions(&before, &after);
        assert!(transitions.contains(&GameTransition::GameEnded));
    }

    #[test]
    fn test_derive_game_abandoned() {
        let before = view(GameState::Lobby, None);
        let after = view(GameState::Abandoned, None);
        let transitions = derive_game_transitions(&before, &after);
        assert!(transitions.contains(&GameTransition::GameAbandoned));
        // Should NOT contain GameStarted if moving directly from Lobby to Abandoned
        assert!(!transitions.contains(&GameTransition::GameStarted));
    }

    #[test]
    fn test_derive_turn_change() {
        let before = view(GameState::TrickPlay, Some(0));
        let after = view(GameState::TrickPlay, Some(1));
        let transitions = derive_game_transitions(&before, &after);
        assert!(transitions.contains(&GameTransition::TurnBecame { player_id: 1 }));
    }
}
