//! Tricks repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::tricks_sea as tricks_adapter;
use crate::entities::round_tricks;
use crate::errors::domain::DomainError;

/// Trick domain model
#[derive(Debug, Clone, PartialEq)]
pub struct Trick {
    pub id: i64,
    pub round_id: i64,
    pub trick_no: u8,
    pub lead_suit: Suit,
    pub winner_seat: u8,
    pub created_at: time::OffsetDateTime,
}

/// Suit representation (domain type)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

// Free functions (generic) for trick operations

/// Create a completed trick
pub async fn create_trick(
    txn: &DatabaseTransaction,
    round_id: i64,
    trick_no: u8,
    lead_suit: Suit,
    winner_seat: u8,
) -> Result<Trick, DomainError> {
    let dto = tricks_adapter::TrickCreate {
        round_id,
        trick_no,
        lead_suit: lead_suit.into(),
        winner_seat,
    };
    let trick = tricks_adapter::create_trick(txn, dto).await?;
    Ok(Trick::from(trick))
}

/// Find a specific trick by round and trick number
pub async fn find_by_round_and_trick<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
    trick_no: u8,
) -> Result<Option<Trick>, DomainError> {
    let trick = tricks_adapter::find_by_round_and_trick(conn, round_id, trick_no).await?;
    Ok(trick.map(Trick::from))
}

/// Find all tricks for a round (ordered by trick_no)
pub async fn find_all_by_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Vec<Trick>, DomainError> {
    let tricks = tricks_adapter::find_all_by_round(conn, round_id).await?;
    Ok(tricks.into_iter().map(Trick::from).collect())
}

/// Count completed tricks for a round
pub async fn count_tricks_by_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<u64, DomainError> {
    let count = tricks_adapter::count_tricks_by_round(conn, round_id).await?;
    Ok(count)
}

/// Update trick winner
pub async fn update_winner(
    txn: &DatabaseTransaction,
    trick_id: i64,
    winner_seat: u8,
) -> Result<(), DomainError> {
    tricks_adapter::update_winner(txn, trick_id, winner_seat).await?;
    Ok(())
}

// Conversions between SeaORM models and domain models

impl From<round_tricks::Model> for Trick {
    fn from(model: round_tricks::Model) -> Self {
        Self {
            id: model.id,
            round_id: model.round_id,
            trick_no: model.trick_no as u8,
            lead_suit: Suit::from(model.lead_suit),
            winner_seat: model.winner_seat as u8,
            created_at: model.created_at,
        }
    }
}

impl From<round_tricks::CardSuit> for Suit {
    fn from(cs: round_tricks::CardSuit) -> Self {
        match cs {
            round_tricks::CardSuit::Clubs => Suit::Clubs,
            round_tricks::CardSuit::Diamonds => Suit::Diamonds,
            round_tricks::CardSuit::Hearts => Suit::Hearts,
            round_tricks::CardSuit::Spades => Suit::Spades,
        }
    }
}

impl From<Suit> for round_tricks::CardSuit {
    fn from(s: Suit) -> Self {
        match s {
            Suit::Clubs => round_tricks::CardSuit::Clubs,
            Suit::Diamonds => round_tricks::CardSuit::Diamonds,
            Suit::Hearts => round_tricks::CardSuit::Hearts,
            Suit::Spades => round_tricks::CardSuit::Spades,
        }
    }
}
