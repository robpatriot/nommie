//! SeaORM adapter for game repository - generic over ConnectionTrait.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, NotSet, QueryFilter, Set,
};

use crate::entities::games;

pub mod dto;

pub use dto::{GameCreate, GameUpdateMetadata, GameUpdateRound, GameUpdateState};

// Adapter functions return DbErr; repos layer maps to DomainError via From<DbErr>.

/// Helper: Apply optimistic update with lock version check, then refetch.
///
/// This consolidates the repetitive pattern:
/// - Adds lock_version increment and updated_at to the update
/// - Filters by id and current_lock_version
/// - Checks rows_affected to distinguish NotFound vs OptimisticLock
/// - Refetches and returns the updated model
///
/// The caller provides a closure that configures entity-specific columns.
async fn optimistic_update_then_fetch<C, F>(
    conn: &C,
    id: i64,
    current_lock_version: i32,
    configure_update: F,
) -> Result<games::Model, sea_orm::DbErr>
where
    C: ConnectionTrait + Send + Sync,
    F: FnOnce(sea_orm::UpdateMany<games::Entity>) -> sea_orm::UpdateMany<games::Entity>,
{
    use sea_orm::sea_query::Expr;

    let now = time::OffsetDateTime::now_utc();

    // Apply caller's column updates, then add lock_version increment and filters
    let result = configure_update(games::Entity::update_many())
        .col_expr(games::Column::UpdatedAt, Expr::val(now).into())
        .col_expr(
            games::Column::LockVersion,
            Expr::col(games::Column::LockVersion).add(1),
        )
        .filter(games::Column::Id.eq(id))
        .filter(games::Column::LockVersion.eq(current_lock_version))
        .exec(conn)
        .await?;

    if result.rows_affected == 0 {
        // Either the game doesn't exist or the lock version doesn't match
        // Check if game exists to distinguish between NotFound and OptimisticLock
        let game = games::Entity::find_by_id(id).one(conn).await?;
        if let Some(game) = game {
            // Lock version mismatch - build structured payload
            let payload = format!(
                "OPTIMISTIC_LOCK:{{\"expected\":{},\"actual\":{}}}",
                current_lock_version, game.lock_version
            );
            return Err(sea_orm::DbErr::Custom(payload));
        } else {
            return Err(sea_orm::DbErr::RecordNotFound("Game not found".to_string()));
        }
    }

    // Fetch and return the updated game
    games::Entity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Game not found".to_string()))
}

pub async fn find_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
) -> Result<Option<games::Model>, sea_orm::DbErr> {
    games::Entity::find()
        .filter(games::Column::Id.eq(game_id))
        .one(conn)
        .await
}

/// Find game by ID or return RecordNotFound error.
///
/// This is a convenience helper that converts `None` into a DbErr::RecordNotFound,
/// eliminating the repetitive `ok_or_else` pattern when a game must exist.
///
/// # Example
/// ```rust
/// let game = require_game(conn, game_id).await?;
/// // game is guaranteed to exist here
/// ```
pub async fn require_game<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
) -> Result<games::Model, sea_orm::DbErr> {
    find_by_id(conn, game_id)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Game not found".to_string()))
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

    optimistic_update_then_fetch(conn, dto.id, dto.current_lock_version, |update| {
        update.col_expr(
            games::Column::State,
            Expr::val(dto.state).cast_as(Alias::new("game_state")),
        )
    })
    .await
}

pub async fn update_metadata<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    dto: GameUpdateMetadata,
) -> Result<games::Model, sea_orm::DbErr> {
    use sea_orm::sea_query::{Alias, Expr};

    optimistic_update_then_fetch(conn, dto.id, dto.current_lock_version, |update| {
        update
            .col_expr(games::Column::Name, Expr::val(dto.name).into())
            .col_expr(
                games::Column::Visibility,
                Expr::val(dto.visibility).cast_as(Alias::new("game_visibility")),
            )
    })
    .await
}

pub async fn update_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    dto: GameUpdateRound,
) -> Result<games::Model, sea_orm::DbErr> {
    use sea_orm::sea_query::Expr;

    optimistic_update_then_fetch(conn, dto.id, dto.current_lock_version, |mut update| {
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
        update
    })
    .await
}
