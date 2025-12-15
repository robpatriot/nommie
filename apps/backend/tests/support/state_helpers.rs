use backend::domain::rules::PLAYERS;
use backend::domain::state::{GameState, Phase, PlayerId, RoundState};
use backend::domain::Card;

/// Initialize a new round's `GameState` for integration tests.
///
/// Mirrors the behavior of the domain `init_round` helper used in unit tests:
/// - Starts in the Bidding phase
/// - Sets `turn_start`, `turn`, and `leader` to the player to the left of the dealer
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


