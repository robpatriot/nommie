//! Core card-related types: Card, Rank, Suit, Trump

use crate::errors::domain::{DomainError, ValidationKind};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Trump {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
    NoTrumps,
}

impl From<Suit> for Trump {
    fn from(suit: Suit) -> Self {
        match suit {
            Suit::Clubs => Trump::Clubs,
            Suit::Diamonds => Trump::Diamonds,
            Suit::Hearts => Trump::Hearts,
            Suit::Spades => Trump::Spades,
        }
    }
}

impl TryFrom<Trump> for Suit {
    type Error = DomainError;

    fn try_from(trump: Trump) -> Result<Self, Self::Error> {
        match trump {
            Trump::Clubs => Ok(Suit::Clubs),
            Trump::Diamonds => Ok(Suit::Diamonds),
            Trump::Hearts => Ok(Suit::Hearts),
            Trump::Spades => Ok(Suit::Spades),
            Trump::NoTrumps => Err(DomainError::validation(
                ValidationKind::InvalidTrumpConversion,
                "Cannot convert NoTrumps to Suit",
            )),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

// Note: Ord/Eq on Card is only for stable sorting: suit order C<D<H<S then rank order.
// Do not use for trick resolution or game logic comparisons involving trump/lead.
impl Ord for Card {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.suit.cmp(&other.suit) {
            std::cmp::Ordering::Equal => self.rank.cmp(&other.rank),
            ord => ord,
        }
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
