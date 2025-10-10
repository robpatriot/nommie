//! SeaORM adapter for hands repository - generic over ConnectionTrait.

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;

use crate::entities::round_hands;

pub mod dto;

pub use dto::HandCreate;

/// Find a hand by round_id and player_seat
pub async fn find_by_round_and_seat<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
    player_seat: i16,
) -> Result<Option<round_hands::Model>, sea_orm::DbErr> {
    round_hands::Entity::find()
        .filter(round_hands::Column::RoundId.eq(round_id))
        .filter(round_hands::Column::PlayerSeat.eq(player_seat))
        .one(conn)
        .await
}

/// Find all hands for a round
pub async fn find_all_by_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Vec<round_hands::Model>, sea_orm::DbErr> {
    round_hands::Entity::find()
        .filter(round_hands::Column::RoundId.eq(round_id))
        .all(conn)
        .await
}

/// Create a hand for a player
pub async fn create_hand<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    dto: HandCreate,
) -> Result<round_hands::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();

    // Convert Vec<Card> to JSONB
    let cards_json = json!(dto.cards);

    let hand = round_hands::ActiveModel {
        id: sea_orm::NotSet,
        round_id: Set(dto.round_id),
        player_seat: Set(dto.player_seat),
        cards: Set(cards_json),
        created_at: Set(now),
    };

    hand.insert(conn).await
}
