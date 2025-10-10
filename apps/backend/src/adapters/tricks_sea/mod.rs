//! SeaORM adapter for tricks repository - generic over ConnectionTrait.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};

use crate::entities::round_tricks;

pub mod dto;

pub use dto::TrickCreate;

/// Find a trick by round_id and trick_no
pub async fn find_by_round_and_trick<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
    trick_no: i16,
) -> Result<Option<round_tricks::Model>, sea_orm::DbErr> {
    round_tricks::Entity::find()
        .filter(round_tricks::Column::RoundId.eq(round_id))
        .filter(round_tricks::Column::TrickNo.eq(trick_no))
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
pub async fn create_trick<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    dto: TrickCreate,
) -> Result<round_tricks::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();

    let trick = round_tricks::ActiveModel {
        id: sea_orm::NotSet,
        round_id: Set(dto.round_id),
        trick_no: Set(dto.trick_no),
        lead_suit: Set(dto.lead_suit),
        winner_seat: Set(dto.winner_seat),
        created_at: Set(now),
    };

    trick.insert(conn).await
}
