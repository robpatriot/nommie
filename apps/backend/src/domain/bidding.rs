use crate::domain::errors::DomainError;
use crate::domain::state::{GameState, PlayerId};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Bid(pub u8);

pub fn legal_bids(_state: &GameState, _who: PlayerId) -> Vec<Bid> {
    Vec::new()
}

pub fn place_bid(_state: &mut GameState, _who: PlayerId, _bid: Bid) -> Result<(), DomainError> {
    Err(DomainError::InvalidBid)
}


