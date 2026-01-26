use crate::domain::player_view::GameHistory;
use crate::domain::rules::{valid_bid_range, PLAYERS};
use crate::domain::state::{
    next_player, require_dealer, require_hand_size, require_turn, GameState, Phase, PlayerId,
};
use crate::errors::domain::{DomainError, ValidationKind};

/// Result of placing a bid, describing what state changes occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceBidResult {
    /// Phase transitioned to, if bidding completed (None means still in Bidding phase).
    pub phase_transitioned: Option<Phase>,
    /// Winning bidder determined, if bidding completed.
    pub winning_bidder: Option<PlayerId>,
}

/// Result of setting trump, describing what state changes occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetTrumpResult {
    /// Trick number that play starts at (always 1 when trump is set).
    pub trick_no: u8,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Bid(pub u8);

impl Bid {
    pub fn value(self) -> u8 {
        self.0
    }
}

/// Compute legal bids for a player. If phase is not Bidding, returns empty.
/// This function does not enforce turn order; `place_bid` does.
pub fn legal_bids(state: &GameState, _who: PlayerId) -> Vec<Bid> {
    if state.phase != Phase::Bidding {
        return Vec::new();
    }

    let Some(hand_size) = state.hand_size else {
        // Invariant violation, but this is a pure helper; return empty instead of erroring.
        return Vec::new();
    };

    valid_bid_range(hand_size).map(Bid).collect()
}

/// Place a bid for `who`. Requires Bidding phase and being in turn.
pub fn place_bid(
    state: &mut GameState,
    who: PlayerId,
    bid: Bid,
) -> Result<PlaceBidResult, DomainError> {
    if state.phase != Phase::Bidding {
        return Err(DomainError::validation(
            ValidationKind::PhaseMismatch,
            "Phase mismatch",
        ));
    }

    let turn = require_turn(state, "place_bid")?;
    if turn != who {
        return Err(DomainError::validation(
            ValidationKind::OutOfTurn,
            "Out of turn",
        ));
    }

    let hand_size = require_hand_size(state, "place_bid")?;
    let range = valid_bid_range(hand_size);
    if !range.contains(&bid.0) {
        return Err(DomainError::validation(
            ValidationKind::InvalidBid,
            "Invalid bid",
        ));
    }

    let idx = who as usize;
    if state.round.bids[idx].is_some() {
        return Err(DomainError::validation(
            ValidationKind::InvalidBid,
            "Invalid bid",
        ));
    }

    // Dealer bid restriction: if this is the 4th (final) bid, check dealer rule
    let bids_count = state.round.bids.iter().filter(|b| b.is_some()).count();
    if bids_count == 3 {
        // This is the dealer's bid - sum of all bids cannot equal hand_size
        let existing_sum: u8 = state.round.bids.iter().flatten().sum();
        let proposed_sum = existing_sum + bid.0;

        if proposed_sum == hand_size {
            return Err(DomainError::validation(
                ValidationKind::InvalidBid,
                format!(
                    "Dealer cannot bid {}: sum would be {} = hand_size",
                    bid.0, proposed_sum
                ),
            ));
        }
    }

    state.round.bids[idx] = Some(bid.0);

    // Advance turn explicitly. (Safe: Bidding is always actionable.)
    state.turn = Some(next_player(who));

    let mut result = PlaceBidResult {
        phase_transitioned: None,
        winning_bidder: None,
    };

    if state.round.bids.iter().all(|b| b.is_some()) {
        // Determine winning bidder: highest bid; ties resolved by earliest from round start (left of dealer).
        let dealer = require_dealer(state, "place_bid winner_resolution")?;
        let round_start = next_player(dealer);

        let mut best_bid: Option<u8> = None;
        let mut winner: Option<PlayerId> = None;

        for i in 0..PLAYERS as u8 {
            let pid = (round_start + i) % PLAYERS as u8;
            let b = state.round.bids[pid as usize].ok_or_else(|| {
                DomainError::validation_other(
                    "Invariant violated: bid should be present after checking all bids are set",
                )
            })?;

            match best_bid {
                None => {
                    best_bid = Some(b);
                    winner = Some(pid);
                }
                Some(curr) => {
                    if b > curr {
                        best_bid = Some(b);
                        winner = Some(pid);
                    }
                }
            }
        }

        state.round.winning_bidder = winner;
        state.phase = Phase::TrumpSelect;
        state.turn = winner;

        result.phase_transitioned = Some(Phase::TrumpSelect);
        result.winning_bidder = winner;
    }

    Ok(result)
}

/// Set trump; only the winning bidder can call in TrumpSelect phase.
pub fn set_trump(
    state: &mut GameState,
    who: PlayerId,
    trump: crate::domain::Trump,
) -> Result<SetTrumpResult, DomainError> {
    if state.phase != Phase::TrumpSelect {
        return Err(DomainError::validation(
            ValidationKind::PhaseMismatch,
            "Phase mismatch",
        ));
    }

    match state.round.winning_bidder {
        Some(bidder) if bidder == who => {
            let dealer = require_dealer(state, "set_trump")?;
            let round_start = next_player(dealer);

            state.round.trump = Some(trump);

            // First trick is led by player to left of dealer (round start)
            state.leader = Some(round_start);
            state.turn = Some(round_start);
            state.trick_no = Some(1);

            state.round.trick_plays.clear();
            state.round.trick_lead = None;

            state.phase = Phase::Trick { trick_no: 1 };

            Ok(SetTrumpResult { trick_no: 1 })
        }
        _ => Err(DomainError::validation(
            ValidationKind::OutOfTurn,
            "Out of turn",
        )),
    }
}

/// Validate that a player hasn't bid 0 three times in a row.
///
/// According to game rules, a player may bid 0, but cannot do so more than
/// three rounds in a row. After three consecutive 0-bids, they must bid at
/// least 1 in the next round.
pub fn validate_consecutive_zero_bids(
    history: &GameHistory,
    player_seat: u8,
    current_round: u8,
) -> Result<(), DomainError> {
    // Need at least 3 previous rounds to check
    if current_round < 4 {
        return Ok(());
    }

    // Get the last 3 completed rounds
    let recent_rounds: Vec<_> = history
        .rounds
        .iter()
        .filter(|r| r.round_no < current_round) // Only completed rounds
        .rev() // Most recent first
        .take(3)
        .collect();

    // If we don't have 3 rounds yet, allow the bid
    if recent_rounds.len() < 3 {
        return Ok(());
    }

    // Check if all 3 recent rounds have 0 bids from this player
    let all_zeros = recent_rounds
        .iter()
        .all(|round| round.bids[player_seat as usize] == Some(0));

    if all_zeros {
        Err(DomainError::validation(
            ValidationKind::InvalidBid,
            "Cannot bid 0 more than three rounds in a row",
        ))
    } else {
        Ok(())
    }
}
