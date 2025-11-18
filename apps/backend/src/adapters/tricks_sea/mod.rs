//! SeaORM adapter for tricks repository.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::entities::round_tricks;

pub mod dto;

pub use dto::TrickCreate;

/// Find a trick by round_id and trick_no
pub async fn find_by_round_and_trick<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
    trick_no: u8,
) -> Result<Option<round_tricks::Model>, sea_orm::DbErr> {
    round_tricks::Entity::find()
        .filter(round_tricks::Column::RoundId.eq(round_id))
        .filter(round_tricks::Column::TrickNo.eq(trick_no as i16))
        .one(conn)
        .await
}

/// Find all tricks for a round (ordered by trick_no)
pub async fn find_all_by_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Vec<round_tricks::Model>, sea_orm::DbErr> {
    round_tricks::Entity::find()
        .filter(round_tricks::Column::RoundId.eq(round_id))
        .order_by(round_tricks::Column::TrickNo, Order::Asc)
        .all(conn)
        .await
}

/// Count tricks for a round
pub async fn count_tricks_by_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<u64, sea_orm::DbErr> {
    round_tricks::Entity::find()
        .filter(round_tricks::Column::RoundId.eq(round_id))
        .count(conn)
        .await
}

/// Create a trick
pub async fn create_trick(
    txn: &DatabaseTransaction,
    dto: TrickCreate,
) -> Result<round_tricks::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();

    let trick = round_tricks::ActiveModel {
        id: sea_orm::NotSet,
        round_id: Set(dto.round_id),
        trick_no: Set(dto.trick_no as i16),
        lead_suit: Set(dto.lead_suit),
        winner_seat: Set(dto.winner_seat as i16),
        created_at: Set(now),
    };

    trick.insert(txn).await
}

/// Update trick winner
pub async fn update_winner(
    txn: &DatabaseTransaction,
    trick_id: i64,
    winner_seat: u8,
) -> Result<round_tricks::Model, sea_orm::DbErr> {
    let trick = round_tricks::ActiveModel {
        id: Set(trick_id),
        round_id: sea_orm::NotSet,
        trick_no: sea_orm::NotSet,
        lead_suit: sea_orm::NotSet,
        winner_seat: Set(winner_seat as i16),
        created_at: sea_orm::NotSet,
    };

    trick.update(txn).await
}
