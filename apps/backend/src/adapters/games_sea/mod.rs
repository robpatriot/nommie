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
    use sea_orm::sea_query::{Alias, Expr};

    // Update with WHERE id = ? AND lock_version = ? (optimistic locking)
    let now = time::OffsetDateTime::now_utc();

    let result = games::Entity::update_many()
        .col_expr(
            games::Column::State,
            Expr::val(dto.state).cast_as(Alias::new("game_state")),
        )
        .col_expr(games::Column::UpdatedAt, Expr::val(now).into())
        .col_expr(
            games::Column::LockVersion,
            Expr::col(games::Column::LockVersion).add(1),
        )
        .filter(games::Column::Id.eq(dto.id))
        .filter(games::Column::LockVersion.eq(dto.expected_lock_version))
        .exec(conn)
        .await?;

    if result.rows_affected == 0 {
        // Either the game doesn't exist or the lock version doesn't match
        // Check if game exists to distinguish between NotFound and OptimisticLock
        let exists = games::Entity::find_by_id(dto.id).one(conn).await?.is_some();
        if exists {
            return Err(sea_orm::DbErr::Custom(
                "OPTIMISTIC_LOCK: Game was modified by another transaction".to_string(),
            ));
        } else {
            return Err(sea_orm::DbErr::RecordNotFound("Game not found".to_string()));
        }
    }

    // Fetch and return the updated game
    games::Entity::find_by_id(dto.id)
        .one(conn)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Game not found".to_string()))
}

pub async fn update_metadata<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    dto: GameUpdateMetadata,
) -> Result<games::Model, sea_orm::DbErr> {
    use sea_orm::sea_query::{Alias, Expr};

    // Update with WHERE id = ? AND lock_version = ? (optimistic locking)
    let now = time::OffsetDateTime::now_utc();

    let result = games::Entity::update_many()
        .col_expr(games::Column::Name, Expr::val(dto.name).into())
        .col_expr(
            games::Column::Visibility,
            Expr::val(dto.visibility).cast_as(Alias::new("game_visibility")),
        )
        .col_expr(games::Column::UpdatedAt, Expr::val(now).into())
        .col_expr(
            games::Column::LockVersion,
            Expr::col(games::Column::LockVersion).add(1),
        )
        .filter(games::Column::Id.eq(dto.id))
        .filter(games::Column::LockVersion.eq(dto.expected_lock_version))
        .exec(conn)
        .await?;

    if result.rows_affected == 0 {
        // Either the game doesn't exist or the lock version doesn't match
        // Check if game exists to distinguish between NotFound and OptimisticLock
        let exists = games::Entity::find_by_id(dto.id).one(conn).await?.is_some();
        if exists {
            return Err(sea_orm::DbErr::Custom(
                "OPTIMISTIC_LOCK: Game was modified by another transaction".to_string(),
            ));
        } else {
            return Err(sea_orm::DbErr::RecordNotFound("Game not found".to_string()));
        }
    }

    // Fetch and return the updated game
    games::Entity::find_by_id(dto.id)
        .one(conn)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Game not found".to_string()))
}

pub async fn update_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    dto: GameUpdateRound,
) -> Result<games::Model, sea_orm::DbErr> {
    use sea_orm::sea_query::Expr;

    // Update with WHERE id = ? AND lock_version = ? (optimistic locking)
    let now = time::OffsetDateTime::now_utc();

    let mut update = games::Entity::update_many();

    // Apply optional field updates
    if let Some(round) = dto.current_round {
        update = update.col_expr(games::Column::CurrentRound, Expr::val(Some(round)).into());
    }
    if let Some(size) = dto.hand_size {
        update = update.col_expr(games::Column::HandSize, Expr::val(Some(size)).into());
    }
    if let Some(pos) = dto.dealer_pos {
        update = update.col_expr(games::Column::DealerPos, Expr::val(Some(pos)).into());
    }

    // Always update updated_at and lock_version
    let result = update
        .col_expr(games::Column::UpdatedAt, Expr::val(now).into())
        .col_expr(
            games::Column::LockVersion,
            Expr::col(games::Column::LockVersion).add(1),
        )
        .filter(games::Column::Id.eq(dto.id))
        .filter(games::Column::LockVersion.eq(dto.expected_lock_version))
        .exec(conn)
        .await?;

    if result.rows_affected == 0 {
        // Either the game doesn't exist or the lock version doesn't match
        // Check if game exists to distinguish between NotFound and OptimisticLock
        let exists = games::Entity::find_by_id(dto.id).one(conn).await?.is_some();
        if exists {
            return Err(sea_orm::DbErr::Custom(
                "OPTIMISTIC_LOCK: Game was modified by another transaction".to_string(),
            ));
        } else {
            return Err(sea_orm::DbErr::RecordNotFound("Game not found".to_string()));
        }
    }

    // Fetch and return the updated game
    games::Entity::find_by_id(dto.id)
        .one(conn)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Game not found".to_string()))
}
