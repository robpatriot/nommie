//! Bids repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::bids_sea as bids_adapter;
use crate::entities::round_bids;
use crate::errors::domain::DomainError;

/// Bid domain model
#[derive(Debug, Clone, PartialEq)]
pub struct Bid {
    pub id: i64,
    pub round_id: i64,
    pub player_seat: u8,
    pub bid_value: u8,
    pub bid_order: u8,
    pub created_at: time::OffsetDateTime,
}

// Free functions (generic) for bid operations

/// Create a bid for a player
pub async fn create_bid(
    txn: &DatabaseTransaction,
    round_id: i64,
    player_seat: u8,
    bid_value: u8,
    bid_order: u8,
) -> Result<Bid, DomainError> {
    let dto = bids_adapter::BidCreate {
        round_id,
        player_seat,
        bid_value,
        bid_order,
    };
    let bid = bids_adapter::create_bid(txn, dto).await?;
    Ok(Bid::from(bid))
}

/// Find all bids for a round
pub async fn find_all_by_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Vec<Bid>, DomainError> {
    let bids = bids_adapter::find_all_by_round(conn, round_id).await?;
    Ok(bids.into_iter().map(Bid::from).collect())
}

/// Count how many bids have been placed for a round
pub async fn count_bids_by_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<u64, DomainError> {
    let count = bids_adapter::count_bids_by_round(conn, round_id).await?;
    Ok(count)
}

/// Find the winning bid for a round (highest bid, tie-breaker by bid_order)
pub async fn find_winning_bid<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Option<Bid>, DomainError> {
    let bid = bids_adapter::find_winning_bid(conn, round_id).await?;
    Ok(bid.map(Bid::from))
}

// Conversions between SeaORM models and domain models

impl From<round_bids::Model> for Bid {
    fn from(model: round_bids::Model) -> Self {
        Self {
            id: model.id,
            round_id: model.round_id,
            player_seat: model.player_seat as u8,
            bid_value: model.bid_value as u8,
            bid_order: model.bid_order as u8,
            created_at: model.created_at,
        }
    }
}
