//! Hands repository functions for domain layer (generic over ConnectionTrait).

use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};

use crate::adapters::hands_sea as hands_adapter;
use crate::entities::round_hands;
use crate::errors::domain::DomainError;

/// Hand domain model
#[derive(Debug, Clone, PartialEq)]
pub struct Hand {
    pub id: i64,
    pub round_id: i64,
    pub player_seat: i16,
    pub cards: Vec<Card>,
    pub created_at: time::OffsetDateTime,
}

/// Card representation (matches domain Card type)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    pub suit: String,
    pub rank: String,
}

// Free functions (generic) for hand operations

/// Create hands for all players in a round
pub async fn create_hands<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
    hands: Vec<(i16, Vec<Card>)>, // Vec of (player_seat, cards)
) -> Result<Vec<Hand>, DomainError> {
    let mut results = Vec::new();
    for (seat, cards) in hands {
        let dto = hands_adapter::HandCreate {
            round_id,
            player_seat: seat,
            cards,
        };
        let hand = hands_adapter::create_hand(conn, dto).await?;
        results.push(Hand::from(hand));
    }
    Ok(results)
}

/// Find a player's hand for a specific round
pub async fn find_by_round_and_seat<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
    player_seat: i16,
) -> Result<Option<Hand>, DomainError> {
    let hand = hands_adapter::find_by_round_and_seat(conn, round_id, player_seat).await?;
    Ok(hand.map(Hand::from))
}

/// Find all hands for a round (useful for tests/admin)
pub async fn find_all_by_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Vec<Hand>, DomainError> {
    let hands = hands_adapter::find_all_by_round(conn, round_id).await?;
    Ok(hands.into_iter().map(Hand::from).collect())
}

// Conversions between SeaORM models and domain models

impl From<round_hands::Model> for Hand {
    fn from(model: round_hands::Model) -> Self {
        // Deserialize JSONB cards into Vec<Card>
        let cards: Vec<Card> =
            serde_json::from_value(model.cards.clone()).unwrap_or_else(|_| Vec::new());

        Self {
            id: model.id,
            round_id: model.round_id,
            player_seat: model.player_seat,
            cards,
            created_at: model.created_at,
        }
    }
}
