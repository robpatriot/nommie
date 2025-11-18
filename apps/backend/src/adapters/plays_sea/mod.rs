//! SeaORM adapter for plays repository.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::json;

use crate::entities::trick_plays;

pub mod dto;

pub use dto::PlayCreate;

/// Find all plays for a trick (ordered by play_order)
pub async fn find_all_by_trick<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    trick_id: i64,
) -> Result<Vec<trick_plays::Model>, sea_orm::DbErr> {
    trick_plays::Entity::find()
        .filter(trick_plays::Column::TrickId.eq(trick_id))
        .order_by(trick_plays::Column::PlayOrder, Order::Asc)
        .all(conn)
        .await
}

/// Count plays for a trick
pub async fn count_plays_by_trick<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    trick_id: i64,
) -> Result<u64, sea_orm::DbErr> {
    trick_plays::Entity::find()
        .filter(trick_plays::Column::TrickId.eq(trick_id))
        .count(conn)
        .await
}

/// Create a play
pub async fn create_play(
    txn: &DatabaseTransaction,
    dto: PlayCreate,
) -> Result<trick_plays::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();

    // Convert Card to JSONB
    let card_json = json!(dto.card);

    let play = trick_plays::ActiveModel {
        id: sea_orm::NotSet,
        trick_id: Set(dto.trick_id),
        player_seat: Set(dto.player_seat as i16),
        card: Set(card_json),
        play_order: Set(dto.play_order as i16),
        played_at: Set(now),
    };

    play.insert(txn).await
}
