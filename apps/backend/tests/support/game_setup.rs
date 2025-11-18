// Game setup helpers for integration tests
//
// This module provides helpers for creating games with players in various configurations.

use backend::entities::games::{self, GameState, GameVisibility};
use backend::entities::users;
use backend::error::AppError;
use backend::infra::db_errors::map_db_err;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, NotSet, Set};
use time::OffsetDateTime;

use super::db_memberships::create_test_game_player_with_ready;
use super::test_utils::{test_seed, test_user_sub};

/// Result of a game setup operation
pub struct GameSetup {
    pub game_id: i64,
    pub user_ids: Vec<i64>,
}

/// Create a test game with 4 ready players using test-specific unique seeds.
///
/// This generates deterministic but unique seeds and user subs per test
/// to avoid conflicts when running tests concurrently.
///
/// # Arguments
/// * `conn` - Database connection or transaction
/// * `test_name` - Name of the test for unique seed generation
///
/// # Returns
/// GameSetup with game_id and user_ids
///
/// # Example
/// ```
/// let setup = setup_game_with_players(txn, "full_game_ai").await?;
/// let game_id = setup.game_id;
/// ```
pub async fn setup_game_with_players<C: ConnectionTrait>(
    conn: &C,
    test_name: &str,
) -> Result<GameSetup, AppError> {
    setup_game_with_players_ex(conn, test_name, 4, true, GameVisibility::Private).await
}

/// Create a test game with custom player configuration using test-specific unique seeds.
///
/// **NOTE: This function bypasses the repository layer and uses ActiveModel directly.**
/// This is intentional for:
/// - Performance: Faster setup when creating games with multiple players
/// - Convenience: Creates users and memberships in one function
/// - Complex scenarios: Setting up complete game states (game + players + readiness)
///
/// For simple game creation, use `repos::games::create_game()` instead.
///
/// # Arguments
/// * `conn` - Database connection or transaction
/// * `test_name` - Test name for unique seed generation
/// * `player_count` - Number of players to create
/// * `all_ready` - Whether all players should be marked ready
/// * `visibility` - Game visibility setting
///
/// # Returns
/// GameSetup with game_id and user_ids
///
/// # Example
/// ```
/// let setup = setup_game_with_players_ex(txn, "my_test", 2, false, GameVisibility::Public).await?;
/// ```
pub async fn setup_game_with_players_ex<C: ConnectionTrait>(
    conn: &C,
    test_name: &str,
    player_count: usize,
    all_ready: bool,
    visibility: GameVisibility,
) -> Result<GameSetup, AppError> {
    let now = OffsetDateTime::now_utc();
    let rng_seed = test_seed(test_name);

    // Create users with unique subs
    let mut user_ids = Vec::new();

    for i in 0..player_count {
        let user_sub = test_user_sub(&format!("{}_player_{}", test_name, i));

        let user = users::ActiveModel {
            id: NotSet,
            sub: Set(user_sub),
            username: Set(Some(format!("player{i}_{rng_seed}"))),
            is_ai: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let inserted_user = user.insert(conn).await?;
        user_ids.push(inserted_user.id);
    }

    // Create game
    let game = games::ActiveModel {
        visibility: Set(visibility),
        state: Set(GameState::Lobby),
        rules_version: Set("nommie-1.0.0".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        rng_seed: Set(Some(rng_seed)),
        lock_version: Set(1),
        ..Default::default()
    };

    let inserted_game = games::Entity::insert(game)
        .exec(conn)
        .await
        .map_err(|e| AppError::from(map_db_err(e)))?;

    let game_id = inserted_game.last_insert_id;

    // Create game_players (memberships)
    for (i, user_id) in user_ids.iter().enumerate() {
        create_test_game_player_with_ready(conn, game_id, *user_id, i as u8, all_ready).await?;
    }

    Ok(GameSetup { game_id, user_ids })
}
