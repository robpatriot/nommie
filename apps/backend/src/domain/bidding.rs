use crate::domain::errors::DomainError;
use crate::domain::rules::valid_bid_range;
use crate::domain::state::{advance_turn, GameState, Phase, PlayerId};
use crate::domain::rules::PLAYERS;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Bid(pub u8);

impl Bid {
    pub fn value(self) -> u8 { self.0 }
}

/// Compute legal bids for a player. If phase is not Bidding, returns empty.
/// This function does not enforce turn order; `place_bid` does.
pub fn legal_bids(state: &GameState, _who: PlayerId) -> Vec<Bid> {
    if state.phase != Phase::Bidding {
        return Vec::new();
    }
    valid_bid_range(state.hand_size)
        .map(Bid)
        .collect()
}

/// Place a bid for `who`. Requires Bidding phase and being in turn.
pub fn place_bid(state: &mut GameState, who: PlayerId, bid: Bid) -> Result<(), DomainError> {
    if state.phase != Phase::Bidding {
        return Err(DomainError::PhaseMismatch);
    }
    if state.turn != who {
        return Err(DomainError::OutOfTurn);
    }
    let range = valid_bid_range(state.hand_size);
    if !range.contains(&bid.0) {
        return Err(DomainError::InvalidBid);
    }
    let idx = who as usize;
    if state.round.bids[idx].is_some() {
        return Err(DomainError::InvalidBid);
    }
    state.round.bids[idx] = Some(bid.0);
    // Advance turn regardless; if all bids are set, determine winner and advance phase
    advance_turn(state);

    if state.round.bids.iter().all(|b| b.is_some()) {
        // Determine winning bidder: highest bid; ties resolved by earliest from turn_start
        let start = state.turn_start;
        let mut best_bid: Option<u8> = None;
        let mut winner: Option<PlayerId> = None;
        for i in 0..PLAYERS as u8 {
            let pid = ((start as u16 + i as u16) % PLAYERS as u16) as u8;
            let b = state.round.bids[pid as usize].unwrap();
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
    }

    Ok(())
}

/// Set trump suit; only the winning bidder can call in TrumpSelect phase.
pub fn set_trump(state: &mut GameState, who: PlayerId, suit: crate::domain::cards::Suit) -> Result<(), DomainError> {
    if state.phase != Phase::TrumpSelect {
        return Err(DomainError::PhaseMismatch);
    }
    match state.round.winning_bidder {
        Some(bidder) if bidder == who => {
            state.round.trump = Some(suit);
            state.leader = bidder;
            state.turn = bidder;
            state.trick_no = 1;
            state.round.trick_plays.clear();
            state.round.trick_lead = None;
            state.phase = Phase::Trick { trick_no: state.trick_no };
            Ok(())
        }
        _ => Err(DomainError::OutOfTurn),
    }
}


