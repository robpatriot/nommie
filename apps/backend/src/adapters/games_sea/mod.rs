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
