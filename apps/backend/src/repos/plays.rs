//! Plays repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};
use serde::{Deserialize, Serialize};

use crate::adapters::plays_sea as plays_adapter;
use crate::entities::trick_plays;
use crate::errors::domain::DomainError;

/// Play domain model (a single card played in a trick)
#[derive(Debug, Clone, PartialEq)]
pub struct Play {
    pub id: i64,
    pub trick_id: i64,
    pub player_seat: i16,
    pub card: Card,
    pub play_order: i16,
    pub played_at: time::OffsetDateTime,
}

/// Card representation (matches domain Card type)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    pub suit: String,
    pub rank: String,
}

// Free functions (generic) for play operations

/// Create a card play
pub async fn create_play(
    txn: &DatabaseTransaction,
    trick_id: i64,
    player_seat: i16,
    card: Card,
    play_order: i16,
) -> Result<Play, DomainError> {
    let dto = plays_adapter::PlayCreate {
        trick_id,
        player_seat,
        card,
        play_order,
    };
    let play = plays_adapter::create_play(txn, dto).await?;
    Ok(Play::from(play))
}

/// Find all plays for a trick (ordered by play_order)
pub async fn find_all_by_trick<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    trick_id: i64,
) -> Result<Vec<Play>, DomainError> {
    let plays = plays_adapter::find_all_by_trick(conn, trick_id).await?;
    Ok(plays.into_iter().map(Play::from).collect())
}

/// Count plays for a trick
pub async fn count_plays_by_trick<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    trick_id: i64,
) -> Result<u64, DomainError> {
    let count = plays_adapter::count_plays_by_trick(conn, trick_id).await?;
    Ok(count)
}

// Conversions between SeaORM models and domain models

impl From<trick_plays::Model> for Play {
    fn from(model: trick_plays::Model) -> Self {
        // Deserialize JSONB card
        let card: Card = serde_json::from_value(model.card.clone()).unwrap_or_else(|_| Card {
            suit: "UNKNOWN".to_string(),
            rank: "UNKNOWN".to_string(),
        });

        Self {
            id: model.id,
            trick_id: model.trick_id,
            player_seat: model.player_seat,
            card,
            play_order: model.play_order,
            played_at: model.played_at,
        }
    }
}
