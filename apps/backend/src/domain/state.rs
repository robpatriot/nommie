use crate::domain::rules::PLAYERS;
use crate::domain::{Card, Suit, Trump};
use crate::errors::domain::DomainError;

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
    /// Last completed trick (4 cards) for display purposes.
    pub last_trick: Option<Vec<(PlayerId, Card)>>,
    /// Summary of the most recently completed round (if any).
    pub previous_round: Option<PreviousRound>,
}

impl RoundState {
    pub fn empty() -> Self {
        Self {
            trick_plays: Vec::with_capacity(4),
            trick_lead: None,
            tricks_won: [0; PLAYERS],
            trump: None,
            bids: [None, None, None, None],
            winning_bidder: None,
            last_trick: None,
            previous_round: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreviousRound {
    pub round_no: u8,
    pub hand_size: u8,
    pub tricks_won: [u8; PLAYERS],
    pub bids: [Option<u8>; PLAYERS],
}

/// Entire game/round container, sufficient for pure domain operations.
#[derive(Debug, Clone)]
pub struct GameState {
    /// Current phase of the round.
    pub phase: Phase,
    /// Round number (1..=26 according to schedule).
    pub round_no: Option<u8>,
    /// Convenience: hand size for this round.
    pub hand_size: Option<u8>,
    /// Players' hands.
    pub hands: [Vec<Card>; PLAYERS],
    /// Dealer seat for the current round.
    /// - None in Init / (optionally) GameOver or any state where dealer is not meaningful.
    pub dealer: Option<PlayerId>,
    /// Player whose turn it is to act.
    /// - Some(seat) when someone is expected to act
    /// - None when nobody can act (Init, Scoring, Complete, GameOver, etc.)
    pub turn: Option<PlayerId>,
    /// Player who leads the current trick (only meaningful in Trick phase).
    pub leader: Option<PlayerId>,
    /// Current trick number (1-based) within the round (only meaningful in Trick phase).
    pub trick_no: Option<u8>,
    /// Cumulative scores across rounds.
    pub scores_total: [i16; PLAYERS],
    /// Per-round container.
    pub round: RoundState,
}

/// Seat / turn math helpers (4 fixed seats: 0..=3).
///
/// These live in `domain` so every layer (services, repos, GameFlow, views)
/// shares a single source of truth for rotation and "who acts next".
///
/// Clockwise direction is positive (+1).
/// Counter-clockwise direction is negative (-1).
#[inline]
pub fn seat_offset(seat: PlayerId, delta: i8) -> PlayerId {
    let seat_i = seat as i16;
    let delta_i = delta as i16;
    ((seat_i + delta_i).rem_euclid(4)) as PlayerId
}

/// Returns the next player clockwise (0 → 1 → 2 → 3 → 0).
#[inline]
pub fn next_player(p: PlayerId) -> PlayerId {
    seat_offset(p, 1)
}

/// Returns the previous player counter-clockwise (0 ← 1 ← 2 ← 3 ← 0).
#[inline]
pub fn prev_player(p: PlayerId) -> PlayerId {
    seat_offset(p, -1)
}

/// Round-start seat (player to the left of the dealer).
#[inline]
pub fn round_start_seat(dealer: PlayerId) -> PlayerId {
    next_player(dealer)
}

/// Returns the seat `n` steps clockwise from `start`.
#[inline]
pub fn nth_from(start: PlayerId, n: u8) -> PlayerId {
    seat_offset(start, n as i8)
}

/// Dealer position for a 1-based round number.
///
/// Round 1 → starting_dealer  
/// Round 2 → starting_dealer + 1 (mod 4)
#[inline]
pub fn dealer_for_round(starting_dealer: PlayerId, round_no: u8) -> PlayerId {
    debug_assert!(round_no >= 1, "round_no is 1-based and must be >= 1");
    let steps = round_no.saturating_sub(1);
    nth_from(starting_dealer, steps)
}

/// Expected bidder seat during bidding.
///
/// Bidding starts at left-of-dealer, then rotates clockwise by `bid_count`.
#[inline]
pub fn expected_bidder(dealer: PlayerId, bid_count: u8) -> PlayerId {
    seat_offset(dealer, 1 + bid_count as i8)
}

/// Expected actor seat during a trick.
///
/// `first_player` is the trick leader; `play_count` is how many cards
/// have already been played into the trick.
#[inline]
pub fn expected_actor(first_player: PlayerId, play_count: u8) -> PlayerId {
    nth_from(first_player, play_count)
}

pub fn require_round_no(state: &GameState, ctx: &'static str) -> Result<u8, DomainError> {
    state.round_no.ok_or_else(|| {
        DomainError::validation_other(format!("Invariant violated: round_no must be set ({ctx})"))
    })
}

pub fn require_hand_size(state: &GameState, ctx: &'static str) -> Result<u8, DomainError> {
    state.hand_size.ok_or_else(|| {
        DomainError::validation_other(format!("Invariant violated: hand_size must be set ({ctx})"))
    })
}

pub fn require_dealer(state: &GameState, ctx: &'static str) -> Result<PlayerId, DomainError> {
    state.dealer.ok_or_else(|| {
        DomainError::validation_other(format!("Invariant violated: dealer must be set ({ctx})"))
    })
}

pub fn require_turn(state: &GameState, ctx: &'static str) -> Result<PlayerId, DomainError> {
    state.turn.ok_or_else(|| {
        DomainError::validation_other(format!("Invariant violated: turn must be set ({ctx})"))
    })
}

pub fn require_leader(state: &GameState, ctx: &'static str) -> Result<PlayerId, DomainError> {
    state.leader.ok_or_else(|| {
        DomainError::validation_other(format!("Invariant violated: leader must be set ({ctx})"))
    })
}

pub fn require_trick_no(state: &GameState, ctx: &'static str) -> Result<u8, DomainError> {
    state.trick_no.ok_or_else(|| {
        DomainError::validation_other(format!("Invariant violated: trick_no must be set ({ctx})"))
    })
}
