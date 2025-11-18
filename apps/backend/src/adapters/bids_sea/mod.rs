//! SeaORM adapter for bids repository.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::entities::round_bids;

pub mod dto;

pub use dto::BidCreate;

/// Find all bids for a round
pub async fn find_all_by_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Vec<round_bids::Model>, sea_orm::DbErr> {
    round_bids::Entity::find()
        .filter(round_bids::Column::RoundId.eq(round_id))
        .order_by(round_bids::Column::BidOrder, Order::Asc)
        .all(conn)
        .await
}

/// Count bids for a round
pub async fn count_bids_by_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<u64, sea_orm::DbErr> {
    round_bids::Entity::find()
        .filter(round_bids::Column::RoundId.eq(round_id))
        .count(conn)
        .await
}

/// Find the winning bid (highest bid_value, tie-breaker by bid_order ascending)
pub async fn find_winning_bid<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Option<round_bids::Model>, sea_orm::DbErr> {
    round_bids::Entity::find()
        .filter(round_bids::Column::RoundId.eq(round_id))
        .order_by(round_bids::Column::BidValue, Order::Desc)
        .order_by(round_bids::Column::BidOrder, Order::Asc)
        .one(conn)
        .await
}

/// Create a bid
pub async fn create_bid(
    txn: &DatabaseTransaction,
    dto: BidCreate,
) -> Result<round_bids::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();

    let bid = round_bids::ActiveModel {
        id: sea_orm::NotSet,
        round_id: Set(dto.round_id),
        player_seat: Set(dto.player_seat as i16),
        bid_value: Set(dto.bid_value as i16),
        bid_order: Set(dto.bid_order as i16),
        created_at: Set(now),
    };

    bid.insert(txn).await
}
