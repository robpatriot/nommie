use backend::db::require_db;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::error::AppError;
use backend::infra::db_errors::map_db_err;
use backend::state::app_state::AppState;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};

// Insert a minimal valid games row using txn-aware connection
pub async fn insert_minimal_game_for_test<C: ConnectionTrait>(
    conn: &C,
    name: &str,
) -> Result<(), AppError> {
    let now = time::OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        created_by: Set(None),
        visibility: Set(GameVisibility::Private),
        state: Set(GameState::Lobby),
        created_at: Set(now),
        updated_at: Set(now),
        started_at: Set(None),
        ended_at: Set(None),
        name: Set(Some(name.to_string())),
        join_code: Set(None),
        rules_version: Set("nommie-1.0.0".to_string()),
        rng_seed: Set(None),
        current_round: Set(None),
        starting_dealer_pos: Set(None),
        current_trick_no: Set(0i16),
        current_round_id: Set(None),
        lock_version: Set(0),
        ..Default::default()
    };

    games::Entity::insert(game)
        .exec(conn)
        .await
        .map(|_| ())
        .map_err(|e| AppError::from(map_db_err(e)))
}

// Delete by name via txn-aware connection; returns affected count
pub async fn delete_games_by_name<C: ConnectionTrait>(
    conn: &C,
    name: &str,
) -> Result<u64, AppError> {
    let res = games::Entity::delete_many()
        .filter(games::Column::Name.eq(Some(name.to_string())))
        .exec(conn)
        .await
        .map_err(|e| AppError::from(map_db_err(e)))?;
    Ok(res.rows_affected)
}

// Delete by join_code via txn-aware connection; returns affected count
pub async fn delete_games_by_join_code<C: ConnectionTrait>(
    conn: &C,
    join_code: &str,
) -> Result<u64, AppError> {
    let res = games::Entity::delete_many()
        .filter(games::Column::JoinCode.eq(Some(join_code.to_string())))
        .exec(conn)
        .await
        .map_err(|e| AppError::from(map_db_err(e)))?;
    Ok(res.rows_affected)
}

// Count visibility using a fresh pooled connection
pub async fn count_games_by_name_pool(state: &AppState, name: &str) -> Result<i64, AppError> {
    let db = require_db(state)?;
    let count = games::Entity::find()
        .filter(games::Column::Name.eq(Some(name.to_string())))
        .count(db)
        .await
        .map_err(|e| AppError::from(map_db_err(e)))?;
    Ok(count as i64)
}

// Convenience existence check using fresh pool
pub async fn fetch_one_game_by_name_pool(state: &AppState, name: &str) -> Result<bool, AppError> {
    let db = require_db(state)?;
    let found = games::Entity::find()
        .filter(games::Column::Name.eq(Some(name.to_string())))
        .one(db)
        .await
        .map_err(|e| AppError::from(map_db_err(e)))?;
    Ok(found.is_some())
}
