// Test helpers for game_players (memberships) table operations

use backend::entities::game_players;
use backend::error::AppError;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

/// Create a test game_player (membership) record
///
/// # Arguments
/// * `conn` - Database connection or transaction
/// * `game_id` - ID of the game
/// * `user_id` - ID of the user
/// * `turn_order` - Turn order for the player
///
/// # Returns
/// ID of the created game_player record
pub async fn create_test_game_player(
    conn: &impl ConnectionTrait,
    game_id: i64,
    user_id: i64,
    turn_order: i32,
) -> Result<i64, AppError> {
    create_test_game_player_with_ready(conn, game_id, user_id, turn_order, false).await
}

/// Create a test game_player record with custom ready state
///
/// # Arguments
/// * `conn` - Database connection or transaction
/// * `game_id` - ID of the game
/// * `user_id` - ID of the user
/// * `turn_order` - Turn order for the player
/// * `is_ready` - Whether the player is ready
///
/// # Returns
/// ID of the created game_player record
pub async fn create_test_game_player_with_ready(
    conn: &impl ConnectionTrait,
    game_id: i64,
    user_id: i64,
    turn_order: i32,
    is_ready: bool,
) -> Result<i64, AppError> {
    let now = time::OffsetDateTime::now_utc();
    let game_player = game_players::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(game_id),
        player_kind: Set(game_players::PlayerKind::Human),
        human_user_id: Set(Some(user_id)),
        ai_profile_id: Set(None),
        turn_order: Set(turn_order),
        is_ready: Set(is_ready),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let inserted = game_player.insert(conn).await?;
    Ok(inserted.id)
}

/// Create an AI game player for tests.
pub async fn create_test_ai_game_player(
    conn: &impl ConnectionTrait,
    game_id: i64,
    ai_profile_id: i64,
    turn_order: i32,
    is_ready: bool,
) -> Result<i64, AppError> {
    let now = time::OffsetDateTime::now_utc();
    let game_player = game_players::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(game_id),
        player_kind: Set(game_players::PlayerKind::Ai),
        human_user_id: Set(None),
        ai_profile_id: Set(Some(ai_profile_id)),
        turn_order: Set(turn_order),
        is_ready: Set(is_ready),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let inserted = game_player.insert(conn).await?;
    Ok(inserted.id)
}
