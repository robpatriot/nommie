use crate::domain::player_view::GameHistory;
use crate::domain::rules::{valid_bid_range, PLAYERS};
use crate::domain::state::{advance_turn, GameState, Phase, PlayerId};
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

/// Compute legal bids for a given hand size.
///
/// This is the core domain logic for determining valid bid values.
/// Returns all possible bids (0 to hand_size inclusive) as `Vec<Bid>`.
///
/// Note: This does not check phase, turn order, or dealer restrictions.
/// Those are handled by higher-level functions like `CurrentRoundInfo::legal_bids()`.
pub fn legal_bids_for_hand_size(hand_size: u8) -> Vec<Bid> {
    valid_bid_range(hand_size).map(Bid).collect()
}

/// Place a bid for `who`. Requires Bidding phase and being in turn.
///
/// # Arguments
/// * `state` - Current game state
/// * `who` - Player placing the bid
/// * `bid` - Bid value
/// * `history` - Optional game history for validating consecutive zero bids rule
pub fn place_bid(
    state: &mut GameState,
    who: PlayerId,
    bid: Bid,
    history: Option<&GameHistory>,
) -> Result<PlaceBidResult, DomainError> {
    if state.phase != Phase::Bidding {
        return Err(DomainError::validation(
            ValidationKind::PhaseMismatch,
            "Phase mismatch",
        ));
    }
    if state.turn != who {
        return Err(DomainError::validation(
            ValidationKind::OutOfTurn,
            "Out of turn",
        ));
    }
    let range = valid_bid_range(state.hand_size);
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

    // Validate consecutive zero bids rule (if bidding 0)
    if bid.0 == 0 {
        if let Some(hist) = history {
            validate_consecutive_zero_bids(hist, who, state.round_no)?;
        }
    }

    // Dealer bid restriction: if this is the 4th (final) bid, check dealer rule
    let bids_count = state.round.bids.iter().filter(|b| b.is_some()).count();
    if bids_count == 3 {
        // This is the dealer's bid - sum of all bids cannot equal hand_size
        let existing_sum: u8 = state.round.bids.iter().flatten().sum();
        let proposed_sum = existing_sum + bid.0;

        if proposed_sum == state.hand_size {
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
    // Advance turn regardless; if all bids are set, determine winner and advance phase
    advance_turn(state);

    let mut result = PlaceBidResult {
        phase_transitioned: None,
        winning_bidder: None,
    };

    if state.round.bids.iter().all(|b| b.is_some()) {
        // Determine winning bidder: highest bid; ties resolved by earliest from turn_start
        let start = state.turn_start;
        let mut best_bid: Option<u8> = None;
        let mut winner: Option<PlayerId> = None;
        for i in 0..PLAYERS as u8 {
            let pid = ((start as u16 + i as u16) % PLAYERS as u16) as u8;
            let b = state.round.bids[pid as usize].ok_or_else(|| {
                DomainError::validation_other(
                    "Bid should be present after checking all bids are set",
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
            state.round.trump = Some(trump);
            // First trick is led by player to left of dealer (turn_start)
            state.leader = state.turn_start;
            state.turn = state.turn_start;
            state.trick_no = 1;
            state.round.trick_plays.clear();
            state.round.trick_lead = None;
            state.phase = Phase::Trick {
                trick_no: state.trick_no,
            };
            Ok(SetTrumpResult {
                trick_no: state.trick_no,
            })
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
///
/// # Arguments
///
/// * `history` - Complete game history with all previous rounds
/// * `player_seat` - Seat position (0-3) of the player attempting to bid
/// * `current_round` - The current round number (1-26)
///
/// # Returns
///
/// * `Ok(())` if the player is allowed to bid 0
/// * `Err(DomainError)` if they've already bid 0 in the last 3 rounds
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
            format!(
                "Cannot bid 0 four times in a row. Player at seat {player_seat} has bid 0 in the last 3 rounds"
            ),
        ))
    } else {
        Ok(())
    }
}
