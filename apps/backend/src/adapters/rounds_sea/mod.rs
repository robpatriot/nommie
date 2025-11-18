//! SeaORM adapter for rounds repository.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, QueryFilter,
    Set,
};

use crate::entities::game_rounds;

pub mod dto;

pub use dto::{RoundCreate, RoundUpdateTrump};

/// Find a round by game_id and round_no
pub async fn find_by_game_and_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
    round_no: u8,
) -> Result<Option<game_rounds::Model>, sea_orm::DbErr> {
    game_rounds::Entity::find()
        .filter(game_rounds::Column::GameId.eq(game_id))
        .filter(game_rounds::Column::RoundNo.eq(round_no as i16))
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

/// Find all rounds for a game (ordered by round_no)
pub async fn find_all_by_game<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
) -> Result<Vec<game_rounds::Model>, sea_orm::DbErr> {
    use sea_orm::QueryOrder;

    game_rounds::Entity::find()
        .filter(game_rounds::Column::GameId.eq(game_id))
        .order_by_asc(game_rounds::Column::RoundNo)
        .all(conn)
        .await
}

/// Create a new round
pub async fn create_round(
    txn: &DatabaseTransaction,
    dto: RoundCreate,
) -> Result<game_rounds::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();

    let round = game_rounds::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(dto.game_id),
        round_no: Set(dto.round_no as i16),
        hand_size: Set(dto.hand_size as i16),
        dealer_pos: Set(dto.dealer_pos as i16),
        trump: Set(None),
        created_at: Set(now),
        completed_at: Set(None),
    };

    round.insert(txn).await
}

/// Update trump selection for a round
pub async fn update_trump(
    txn: &DatabaseTransaction,
    dto: RoundUpdateTrump,
) -> Result<game_rounds::Model, sea_orm::DbErr> {
    // Fetch the round
    let round = find_by_id(txn, dto.round_id)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Round not found".to_string()))?;

    // Update trump
    let mut round: game_rounds::ActiveModel = round.into();
    round.trump = Set(Some(dto.trump));

    round.update(txn).await
}

/// Mark a round as completed
pub async fn complete_round(
    txn: &DatabaseTransaction,
    round_id: i64,
) -> Result<game_rounds::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();

    // Fetch the round
    let round = find_by_id(txn, round_id)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Round not found".to_string()))?;

    // Update completed_at
    let mut round: game_rounds::ActiveModel = round.into();
    round.completed_at = Set(Some(now));

    round.update(txn).await
}
