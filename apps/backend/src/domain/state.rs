use crate::domain::rules::PLAYERS;
use crate::domain::{Card, Suit, Trump};

pub type PlayerId = u8; // 0..=3
pub type Seat = u8; // 0..=3, positional alias for PlayerId

/// Overall game progression phases.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Phase {
    /// Game created but not yet started.
    Init,
    /// Players place bids in fixed turn order.
    Bidding,
    /// Winning bidder selects trump suit.
    TrumpSelect,
    /// Playing tricks within the round; `trick_no` is 1-based.
    Trick { trick_no: u8 },
    /// Tally round points.
    Scoring,
    /// Round complete.
    Complete,
    /// All rounds complete.
    GameOver,
}

/// Per-round state that is relevant during bidding, trump, and trick play.
#[derive(Debug, Clone)]
pub struct RoundState {
    /// Ordered plays for the current trick (who, card).
    pub trick_plays: Vec<(PlayerId, Card)>,
    /// Lead suit for the current trick.
    pub trick_lead: Option<Suit>,
    /// Tricks won per player for this round.
    pub tricks_won: [u8; PLAYERS],
    /// Trump for this round (set by winning bidder).
    pub trump: Option<Trump>,
    /// Bids per player.
    pub bids: [Option<u8>; PLAYERS],
    /// Player who won the bidding (once determined).
    pub winning_bidder: Option<PlayerId>,
}

impl RoundState {
    pub fn new() -> Self {
        Self {
            trick_plays: Vec::with_capacity(4),
            trick_lead: None,
            tricks_won: [0; PLAYERS],
            trump: None,
            bids: [None, None, None, None],
            winning_bidder: None,
        }
    }
}

impl Default for RoundState {
    fn default() -> Self {
        Self::new()
    }
}

/// Entire game/round container, sufficient for pure domain operations.
#[derive(Debug, Clone)]
pub struct GameState {
    /// Current phase of the round.
    pub phase: Phase,
    /// Round number (1..=26 according to schedule).
    pub round_no: u8,
    /// Convenience: hand size for this round.
    pub hand_size: u8,
    /// Players' hands.
    pub hands: [Vec<Card>; PLAYERS],
    /// Turn order anchor for this round (e.g., dealer or bidder order start).
    pub turn_start: PlayerId,
    /// Player whose turn it is to act.
    pub turn: PlayerId,
    /// Player who leads the current trick.
    pub leader: PlayerId,
    /// Current trick number (1-based) within the round.
    pub trick_no: u8,
    /// Cumulative scores across rounds.
    pub scores_total: [i16; PLAYERS],
    /// Per-round container.
    pub round: RoundState,
}

/// Return the current player to act.
pub fn current_player(state: &GameState) -> PlayerId {
    state.turn
}

/// Next player in fixed order, wrapping 0..=3.
pub fn next_player(p: PlayerId) -> PlayerId {
    ((p as usize + 1) % PLAYERS) as u8
}

/// Advance the `turn` to the next player in order.
pub fn advance_turn(state: &mut GameState) {
    state.turn = next_player(state.turn);
}

/// Check if a phase transition is valid.
pub fn can_transition_to(from: Phase, to: Phase) -> bool {
    use Phase::*;
    matches!(
        (from, to),
        (Init, Bidding)
            | (Bidding, TrumpSelect)
            | (TrumpSelect, Trick { .. })
            | (Trick { .. }, Trick { .. })
            | (Trick { .. }, Scoring)
            | (Scoring, Complete)
            | (Complete, Bidding) // Next round
            | (Complete, GameOver)
    )
}

/// Initialize a new round's GameState.
pub fn init_round(
    round_no: u8,
    hand_size: u8,
    hands: [Vec<crate::domain::Card>; PLAYERS],
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
        round: RoundState::new(),
    }
}
