use crate::domain::cards::Card;
use crate::domain::state::{GameState, PlayerId, RoundState};
use crate::domain::errors::DomainError;

pub fn legal_moves(_state: &GameState, _who: PlayerId) -> Vec<Card> {
    Vec::new()
}

pub fn play_card(_state: &mut GameState, _who: PlayerId, _card: Card) -> Result<(), DomainError> {
    Err(DomainError::CardNotInHand)
}

pub fn resolve_current_trick(_state: &RoundState) -> Option<PlayerId> {
    None
}


