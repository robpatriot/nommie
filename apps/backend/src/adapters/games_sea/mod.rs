//! SeaORM adapter for game repository.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, NotSet,
    QueryFilter, Set,
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
async fn optimistic_update_then_fetch<F>(
    txn: &DatabaseTransaction,
    id: i64,
    current_lock_version: i32,
    configure_update: F,
) -> Result<games::Model, sea_orm::DbErr>
where
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
        .exec(txn)
        .await?;

    if result.rows_affected == 0 {
        // Either the game doesn't exist or the lock version doesn't match
        // Check if game exists to distinguish between NotFound and OptimisticLock
        let game = games::Entity::find_by_id(id).one(txn).await?;
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
        .one(txn)
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

pub async fn create_game(
    txn: &DatabaseTransaction,
    dto: GameCreate,
) -> Result<games::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();

    // Generate RNG seed from entropy for deterministic-but-unpredictable gameplay
    // This seed is used to derive all game randomness (dealing, AI memory, etc.)
    let rng_seed = rand::random::<i64>();

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
        rng_seed: Set(Some(rng_seed)),
        current_round: NotSet,
        starting_dealer_pos: NotSet,
        current_trick_no: Set(0),
        current_round_id: NotSet,
        lock_version: Set(1),
    };

    game_active.insert(txn).await
}

pub async fn update_state(
    txn: &DatabaseTransaction,
    dto: GameUpdateState,
) -> Result<games::Model, sea_orm::DbErr> {
    use sea_orm::sea_query::{Alias, Expr};

    optimistic_update_then_fetch(txn, dto.id, dto.current_lock_version, |update| {
        // Runtime dispatch based on database backend
        let state_expr = match txn.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                // PostgreSQL needs explicit cast to enum type
                Expr::val(dto.state).cast_as(Alias::new("game_state"))
            }
            _ => {
                // SQLite and others - just use string value
                Expr::val(dto.state).into()
            }
        };
        update.col_expr(games::Column::State, state_expr)
    })
    .await
}

pub async fn update_metadata(
    txn: &DatabaseTransaction,
    dto: GameUpdateMetadata,
) -> Result<games::Model, sea_orm::DbErr> {
    use sea_orm::sea_query::{Alias, Expr};

    optimistic_update_then_fetch(txn, dto.id, dto.current_lock_version, |update| {
        // Runtime dispatch for visibility enum
        let visibility_expr = match txn.get_database_backend() {
            sea_orm::DatabaseBackend::Postgres => {
                // PostgreSQL needs explicit cast to enum type
                Expr::val(dto.visibility).cast_as(Alias::new("game_visibility"))
            }
            _ => {
                // SQLite and others - just use string value
                Expr::val(dto.visibility).into()
            }
        };

        update
            .col_expr(games::Column::Name, Expr::val(dto.name).into())
            .col_expr(games::Column::Visibility, visibility_expr)
    })
    .await
}

pub async fn update_round(
    txn: &DatabaseTransaction,
    dto: GameUpdateRound,
) -> Result<games::Model, sea_orm::DbErr> {
    use sea_orm::sea_query::Expr;

    optimistic_update_then_fetch(txn, dto.id, dto.current_lock_version, |mut update| {
        // Apply optional field updates
        if let Some(round) = dto.current_round {
            update = update.col_expr(games::Column::CurrentRound, Expr::val(Some(round)).into());
        }
        if let Some(pos) = dto.starting_dealer_pos {
            update = update.col_expr(
                games::Column::StartingDealerPos,
                Expr::val(Some(pos)).into(),
            );
        }
        if let Some(trick_no) = dto.current_trick_no {
            update = update.col_expr(games::Column::CurrentTrickNo, Expr::val(trick_no).into());
        }
        update
    })
    .await
}
