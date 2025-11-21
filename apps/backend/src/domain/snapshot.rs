//! Public snapshot API for observing game state without exposing internals.

use serde::{Deserialize, Serialize};

use crate::domain::rules::{valid_bid_range, PLAYERS};
use crate::domain::state::{GameState, Phase, Seat};
use crate::domain::tricks::legal_moves;
use crate::domain::{Card, Trump};

/// Public info about a single seat in the game.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeatPublic {
    pub seat: Seat,
    pub user_id: Option<i64>,
    pub display_name: Option<String>,
    pub is_ai: bool,
    pub is_ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_profile: Option<SeatAiProfilePublic>,
}

impl SeatPublic {
    pub const fn empty(seat: Seat) -> Self {
        Self {
            seat,
            user_id: None,
            display_name: None,
            is_ai: false,
            is_ready: false,
            ai_profile: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeatAiProfilePublic {
    pub name: String,
    pub version: String,
}

/// Game-level header present in all snapshots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameHeader {
    pub round_no: u8,
    pub dealer: Seat,
    pub seating: [SeatPublic; 4],
    pub scores_total: [i16; 4],
    pub host_seat: Seat,
}

/// Top-level snapshot combining header and phase-specific data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub game: GameHeader,
    pub phase: PhaseSnapshot,
}

/// Adjacently tagged union of phase-specific snapshots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase", content = "data")]
pub enum PhaseSnapshot {
    Init,
    Bidding(BiddingSnapshot),
    TrumpSelect(TrumpSelectSnapshot),
    Trick(TrickSnapshot),
    Scoring(ScoringSnapshot),
    Complete(CompleteSnapshot),
    GameOver,
}

/// Shared public round facts (no private hands).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoundPublic {
    pub hand_size: u8,
    pub leader: Seat,
    pub bid_winner: Option<Seat>,
    pub trump: Option<Trump>,
    pub tricks_won: [u8; 4],
    pub bids: [Option<u8>; 4],
}

/// Summary of the previous round for transition UIs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoundResult {
    pub round_no: u8,
    pub hand_size: u8,
    pub tricks_won: [u8; 4],
    pub bids: [Option<u8>; 4],
}

/// Bidding phase snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BiddingSnapshot {
    pub round: RoundPublic,
    pub to_act: Seat,
    pub bids: [Option<u8>; 4],
    pub min_bid: u8,
    pub max_bid: u8,
    /// Last completed trick from previous round (4 cards) for display purposes.
    pub last_trick: Option<Vec<(Seat, Card)>>,
    /// Final state of the last completed round, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_round: Option<RoundResult>,
}

/// Trump selection phase snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrumpSelectSnapshot {
    pub round: RoundPublic,
    pub to_act: Seat,
    pub allowed_trumps: Vec<Trump>,
    /// Last completed trick from previous round (4 cards) for display purposes.
    pub last_trick: Option<Vec<(Seat, Card)>>,
}

/// Trick-playing phase snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrickSnapshot {
    pub round: RoundPublic,
    pub trick_no: u8,
    pub leader: Seat,
    pub current_trick: Vec<(Seat, Card)>,
    pub to_act: Seat,
    pub playable: Vec<Card>,
    /// Last completed trick (4 cards) for display purposes.
    pub last_trick: Option<Vec<(Seat, Card)>>,
}

/// Scoring phase snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoringSnapshot {
    pub round: RoundPublic,
    pub round_scores: [i16; 4],
}

/// Complete phase snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompleteSnapshot {
    pub round: RoundPublic,
}

/// Entry point: produce a snapshot of the current game state.
/// Never panics; produces safe defaults for inconsistent states.
pub fn snapshot(state: &GameState) -> GameSnapshot {
    let game = GameHeader {
        round_no: state.round_no,
        dealer: compute_dealer(state),
        seating: [
            SeatPublic::empty(0),
            SeatPublic::empty(1),
            SeatPublic::empty(2),
            SeatPublic::empty(3),
        ],
        scores_total: state.scores_total,
        host_seat: 0,
    };

    let phase = match state.phase {
        Phase::Init => PhaseSnapshot::Init,
        Phase::Bidding => snapshot_bidding(state),
        Phase::TrumpSelect => snapshot_trump_select(state),
        Phase::Trick { trick_no } => snapshot_trick(state, trick_no),
        Phase::Scoring => snapshot_scoring(state),
        Phase::Complete => snapshot_complete(state),
        Phase::GameOver => PhaseSnapshot::GameOver,
    };

    GameSnapshot { game, phase }
}

fn compute_dealer(state: &GameState) -> Seat {
    // Dealer rotates: round 1 -> seat 0, round 2 -> seat 1, etc.
    if state.round_no == 0 {
        return 0;
    }
    ((state.round_no - 1) % PLAYERS as u8) as Seat
}

fn build_round_public(state: &GameState) -> RoundPublic {
    RoundPublic {
        hand_size: state.hand_size,
        leader: state.leader,
        bid_winner: state.round.winning_bidder,
        trump: state.round.trump,
        tricks_won: state.round.tricks_won,
        bids: state.round.bids,
    }
}

fn snapshot_bidding(state: &GameState) -> PhaseSnapshot {
    let round = build_round_public(state);
    let to_act = state.turn;
    let bids = state.round.bids;
    let range = valid_bid_range(state.hand_size);
    let min_bid = *range.start();
    let max_bid = *range.end();
    let previous_round = state.round.previous_round.as_ref().map(|prev| RoundResult {
        round_no: prev.round_no,
        hand_size: prev.hand_size,
        tricks_won: prev.tricks_won,
        bids: prev.bids,
    });

    PhaseSnapshot::Bidding(BiddingSnapshot {
        round,
        to_act,
        bids,
        min_bid,
        max_bid,
        last_trick: state.round.last_trick.clone(),
        previous_round,
    })
}

fn snapshot_trump_select(state: &GameState) -> PhaseSnapshot {
    let round = build_round_public(state);
    let to_act = state.round.winning_bidder.unwrap_or(0);
    // All trump options including NO_TRUMP are allowed
    let allowed_trumps = vec![
        Trump::Clubs,
        Trump::Diamonds,
        Trump::Hearts,
        Trump::Spades,
        Trump::NoTrump,
    ];

    PhaseSnapshot::TrumpSelect(TrumpSelectSnapshot {
        round,
        to_act,
        allowed_trumps,
        last_trick: state.round.last_trick.clone(),
    })
}

fn snapshot_trick(state: &GameState, trick_no: u8) -> PhaseSnapshot {
    let round = build_round_public(state);
    let leader = state.leader;
    let current_trick: Vec<(Seat, Card)> = state.round.trick_plays.clone();
    let plays_count = current_trick.len() as u8;
    let to_act = if plays_count >= PLAYERS as u8 {
        leader
    } else {
        (leader + plays_count) % PLAYERS as u8
    };
    let playable = legal_moves(state, to_act);

    PhaseSnapshot::Trick(TrickSnapshot {
        round,
        trick_no,
        leader,
        current_trick,
        to_act,
        playable,
        last_trick: state.round.last_trick.clone(),
    })
}

fn snapshot_scoring(state: &GameState) -> PhaseSnapshot {
    let round = build_round_public(state);
    let round_scores = compute_round_scores(state);

    PhaseSnapshot::Scoring(ScoringSnapshot {
        round,
        round_scores,
    })
}

fn snapshot_complete(state: &GameState) -> PhaseSnapshot {
    let round = build_round_public(state);
    PhaseSnapshot::Complete(CompleteSnapshot { round })
}

/// Compute per-round scoring deltas without mutating state.
fn compute_round_scores(state: &GameState) -> [i16; 4] {
    let mut scores = [0i16; 4];
    for (pid, score) in scores.iter_mut().enumerate().take(PLAYERS) {
        let tricks = state.round.tricks_won[pid] as i16;
        let bid_opt = state.round.bids[pid];
        let bonus = match bid_opt {
            Some(b) if b as i16 == tricks => 10,
            _ => 0,
        };
        *score = tricks + bonus;
    }
    scores
}
