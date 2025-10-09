//! SeaORM adapter for game repository - generic over ConnectionTrait.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, NotSet, QueryFilter, Set,
};

use crate::entities::games;

pub mod dto;

pub use dto::{GameCreate, GameUpdateMetadata, GameUpdateRound, GameUpdateState};

// Adapter functions return DbErr; repos layer maps to DomainError via From<DbErr>.

pub async fn find_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
) -> Result<Option<games::Model>, sea_orm::DbErr> {
    games::Entity::find()
        .filter(games::Column::Id.eq(game_id))
        .one(conn)
        .await
}

pub async fn find_by_join_code<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    join_code: &str,
) -> Result<Option<games::Model>, sea_orm::DbErr> {
    games::Entity::find()
        .filter(games::Column::JoinCode.eq(join_code))
        .one(conn)
        .await
}

pub async fn create_game<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    dto: GameCreate,
) -> Result<games::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let game_active = games::ActiveModel {
        id: NotSet,
        created_by: Set(dto.created_by),
        visibility: Set(dto.visibility.unwrap_or(games::GameVisibility::Private)),
        state: Set(games::GameState::Lobby),
        created_at: Set(now),
        updated_at: Set(now),
        started_at: NotSet,
        ended_at: NotSet,
        name: Set(dto.name),
        join_code: Set(Some(dto.join_code)),
        rules_version: Set("1.0".to_string()),
        rng_seed: NotSet,
        current_round: NotSet,
        hand_size: NotSet,
        dealer_pos: NotSet,
        lock_version: Set(1),
    };

    game_active.insert(conn).await
}

pub async fn update_state<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    dto: GameUpdateState,
) -> Result<games::Model, sea_orm::DbErr> {
    // Fetch existing game
    let existing = games::Entity::find_by_id(dto.id)
        .one(conn)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Game not found".to_string()))?;

    // Update state, updated_at, and increment lock_version; keep created_at unchanged
    let mut active: games::ActiveModel = existing.clone().into();
    active.state = Set(dto.state);
    active.updated_at = Set(time::OffsetDateTime::now_utc());
    active.lock_version = Set(existing.lock_version + 1);

    active.update(conn).await
}

pub async fn update_metadata<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    dto: GameUpdateMetadata,
) -> Result<games::Model, sea_orm::DbErr> {
    // Fetch existing game
    let existing = games::Entity::find_by_id(dto.id)
        .one(conn)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Game not found".to_string()))?;

    // Update metadata fields, updated_at, and increment lock_version; keep created_at unchanged
    let mut active: games::ActiveModel = existing.clone().into();
    active.name = Set(dto.name);
    active.visibility = Set(dto.visibility);
    active.updated_at = Set(time::OffsetDateTime::now_utc());
    active.lock_version = Set(existing.lock_version + 1);

    active.update(conn).await
}

pub async fn update_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    dto: GameUpdateRound,
) -> Result<games::Model, sea_orm::DbErr> {
    // Fetch existing game
    let existing = games::Entity::find_by_id(dto.id)
        .one(conn)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Game not found".to_string()))?;

    // Update round fields, updated_at, and increment lock_version; keep created_at unchanged
    let mut active: games::ActiveModel = existing.clone().into();
    if let Some(round) = dto.current_round {
        active.current_round = Set(Some(round));
    }
    if let Some(size) = dto.hand_size {
        active.hand_size = Set(Some(size));
    }
    if let Some(pos) = dto.dealer_pos {
        active.dealer_pos = Set(Some(pos));
    }
    active.updated_at = Set(time::OffsetDateTime::now_utc());
    active.lock_version = Set(existing.lock_version + 1);

    active.update(conn).await
}
