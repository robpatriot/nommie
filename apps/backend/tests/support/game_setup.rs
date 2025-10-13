//! Game setup helpers for integration tests
//!
//! This module provides helpers for creating games with players in various configurations.

use backend::entities::games::{self, GameState, GameVisibility};
use backend::entities::users;
use backend::error::AppError;
use backend::infra::db_errors::map_db_err;
use backend::utils::unique::unique_str;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, NotSet, Set};
use time::OffsetDateTime;

use super::db_memberships::create_test_game_player_with_ready;

/// Result of a game setup operation
pub struct GameSetup {
    pub game_id: i64,
    pub user_ids: Vec<i64>,
}

/// Options for customizing game setup
#[derive(Debug, Clone)]
pub struct GameSetupOptions {
    pub player_count: usize,
    pub rng_seed: Option<i64>,
    pub all_ready: bool,
    pub visibility: GameVisibility,
}

impl Default for GameSetupOptions {
    fn default() -> Self {
        Self {
            player_count: 4,
            rng_seed: Some(12345),
            all_ready: true,
            visibility: GameVisibility::Private,
        }
    }
}

impl GameSetupOptions {
    pub fn with_player_count(mut self, count: usize) -> Self {
        self.player_count = count;
        self
    }

    pub fn with_rng_seed(mut self, seed: i64) -> Self {
        self.rng_seed = Some(seed);
        self
    }

    pub fn with_ready(mut self, ready: bool) -> Self {
        self.all_ready = ready;
        self
    }

    pub fn with_visibility(mut self, visibility: GameVisibility) -> Self {
        self.visibility = visibility;
        self
    }
}

/// Create a test game with 4 ready players.
///
/// This is the most common test setup - creates a game with 4 players all marked ready.
///
/// # Arguments
/// * `conn` - Database connection or transaction
/// * `rng_seed` - Seed for deterministic RNG (used for dealing cards)
///
/// # Returns
/// GameSetup with game_id and user_ids
///
/// # Example
/// ```
/// let setup = setup_game_with_players(txn, 12345).await?;
/// let game_id = setup.game_id;
/// ```
pub async fn setup_game_with_players<C: ConnectionTrait>(
    conn: &C,
    rng_seed: i64,
) -> Result<GameSetup, AppError> {
    setup_game_with_options(conn, GameSetupOptions::default().with_rng_seed(rng_seed)).await
}

/// Create a test game with custom configuration options.
///
/// Provides full control over player count, readiness, visibility, and RNG seed.
///
/// # Arguments
/// * `conn` - Database connection or transaction
/// * `options` - Configuration options for the game
///
/// # Returns
/// GameSetup with game_id and user_ids
///
/// # Example
/// ```
/// let options = GameSetupOptions::default()
///     .with_player_count(4)
///     .with_rng_seed(42)
///     .with_ready(false);
/// let setup = setup_game_with_options(txn, options).await?;
/// ```
pub async fn setup_game_with_options<C: ConnectionTrait>(
    conn: &C,
    options: GameSetupOptions,
) -> Result<GameSetup, AppError> {
    let now = OffsetDateTime::now_utc();

    // Create users with unique subs
    let mut user_ids = Vec::new();
    let seed_suffix = options.rng_seed.unwrap_or(0);

    for i in 0..options.player_count {
        let user = users::ActiveModel {
            id: NotSet,
            sub: Set(unique_str(&format!("test_user_{seed_suffix}_{i}"))),
            username: Set(Some(format!("player{i}_{seed_suffix}"))),
            is_ai: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let inserted_user = user.insert(conn).await?;
        user_ids.push(inserted_user.id);
    }

    // Create game
    let game = games::ActiveModel {
        visibility: Set(options.visibility),
        state: Set(GameState::Lobby),
        rules_version: Set("nommie-1.0.0".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        rng_seed: Set(options.rng_seed),
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
        create_test_game_player_with_ready(conn, game_id, *user_id, i as i32, options.all_ready)
            .await?;
    }

    Ok(GameSetup { game_id, user_ids })
}
