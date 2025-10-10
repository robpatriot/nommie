//! SeaORM adapter for rounds repository - generic over ConnectionTrait.

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::entities::game_rounds;

pub mod dto;

pub use dto::{RoundCreate, RoundUpdateTrump};

/// Find a round by game_id and round_no
pub async fn find_by_game_and_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
    round_no: i16,
) -> Result<Option<game_rounds::Model>, sea_orm::DbErr> {
    game_rounds::Entity::find()
        .filter(game_rounds::Column::GameId.eq(game_id))
        .filter(game_rounds::Column::RoundNo.eq(round_no))
        .one(conn)
        .await
}

/// Find a round by ID
pub async fn find_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Option<game_rounds::Model>, sea_orm::DbErr> {
    game_rounds::Entity::find_by_id(round_id).one(conn).await
}

/// Create a new round
pub async fn create_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    dto: RoundCreate,
) -> Result<game_rounds::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();

    let round = game_rounds::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(dto.game_id),
        round_no: Set(dto.round_no),
        hand_size: Set(dto.hand_size),
        dealer_pos: Set(dto.dealer_pos),
        trump: Set(None),
        created_at: Set(now),
        completed_at: Set(None),
    };

    round.insert(conn).await
}

/// Update trump selection for a round
pub async fn update_trump<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    dto: RoundUpdateTrump,
) -> Result<game_rounds::Model, sea_orm::DbErr> {
    // Fetch the round
    let round = find_by_id(conn, dto.round_id)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Round not found".to_string()))?;

    // Update trump
    let mut round: game_rounds::ActiveModel = round.into();
    round.trump = Set(Some(dto.trump));

    round.update(conn).await
}

/// Mark a round as completed
pub async fn complete_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<game_rounds::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();

    // Fetch the round
    let round = find_by_id(conn, round_id)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Round not found".to_string()))?;

    // Update completed_at
    let mut round: game_rounds::ActiveModel = round.into();
    round.completed_at = Set(Some(now));

    round.update(conn).await
}
