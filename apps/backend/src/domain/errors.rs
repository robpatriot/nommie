use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    InvalidBid,
    MustFollowSuit,
    CardNotInHand,
    OutOfTurn,
    PhaseMismatch,
    ParseCard(String),
    InvalidTrumpConversion,
    Other(String),
}

impl Display for DomainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            DomainError::InvalidBid => write!(f, "invalid bid"),
            DomainError::MustFollowSuit => write!(f, "must follow suit"),
            DomainError::CardNotInHand => write!(f, "card not in hand"),
            DomainError::OutOfTurn => write!(f, "out of turn"),
            DomainError::PhaseMismatch => write!(f, "phase mismatch"),
            DomainError::ParseCard(s) => write!(f, "parse card: {s}"),
            DomainError::InvalidTrumpConversion => write!(f, "cannot convert NoTrump to Suit"),
            DomainError::Other(s) => write!(f, "domain error: {s}"),
        }
    }
}

impl Error for DomainError {}
