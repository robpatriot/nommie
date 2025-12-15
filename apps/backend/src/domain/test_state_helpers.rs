//! Test-only game state helper for domain unit tests.

#[cfg(test)]
pub use state_helpers::init_round;

#[cfg(test)]
mod state_helpers {
    // Local version that uses crate:: paths for unit tests
    use crate::domain::rules::PLAYERS;
    use crate::domain::state::{GameState, Phase, PlayerId, RoundState};
    use crate::domain::Card;

    /// Initialize a new round's GameState for testing.
    ///
    /// This is a test helper that creates a `GameState` with sensible defaults
    /// for testing. The state starts in the Bidding phase with the turn set to
    /// the player to the left of the dealer.
    pub fn init_round(
        round_no: u8,
        hand_size: u8,
        hands: [Vec<Card>; PLAYERS],
        dealer: PlayerId,
        scores_total: [i16; PLAYERS],
    ) -> GameState {
        let turn_start = next_player(dealer);
        GameState {
            phase: Phase::Bidding,
            round_no,
            hand_size,
            hands,
            turn_start,
            turn: turn_start,
            leader: turn_start,
            trick_no: 0,
            scores_total,
            round: RoundState::empty(),
        }
    }

    /// Next player in fixed order, wrapping 0..=3.
    fn next_player(p: PlayerId) -> PlayerId {
        ((p as usize + 1) % PLAYERS) as u8
    }
}
