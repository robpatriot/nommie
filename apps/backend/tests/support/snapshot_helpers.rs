//! Test helpers for game snapshot integration tests
//!
//! Provides utilities for:
//! - Creating minimal games with specific lock versions and states

use backend::db::txn::SharedTxn;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::error::AppError;
use sea_orm::{ActiveModelTrait, Set};
use time::OffsetDateTime;

/// Result of creating a game for snapshot testing
pub struct SnapshotGameSetup {
    pub game_id: i64,
    pub lock_version: i32,
}

/// Options for customizing snapshot game creation
#[derive(Debug, Clone)]
pub struct SnapshotGameOptions {
    pub state: GameState,
    pub visibility: GameVisibility,
    pub lock_version: i32,
}

impl Default for SnapshotGameOptions {
    fn default() -> Self {
        Self {
            state: GameState::Bidding,
            visibility: GameVisibility::Public,
            lock_version: 1,
        }
    }
}

impl SnapshotGameOptions {
    pub fn with_state(mut self, state: GameState) -> Self {
        self.state = state;
        self
    }

    pub fn with_visibility(mut self, visibility: GameVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_lock_version(mut self, version: i32) -> Self {
        self.lock_version = version;
        self
    }
}

/// Create a minimal game for snapshot testing with specific lock version and state
///
/// # Arguments
/// * `shared` - Shared transaction to use for creation
/// * `options` - Configuration options for the game
///
/// # Returns
/// SnapshotGameSetup with game_id and lock_version
///
/// # Example
/// ```
/// let options = SnapshotGameOptions::default()
///     .with_state(GameState::Bidding)
///     .with_lock_version(5);
/// let game = create_snapshot_game(&shared, options).await?;
/// ```
pub async fn create_snapshot_game(
    shared: &SharedTxn,
    options: SnapshotGameOptions,
) -> Result<SnapshotGameSetup, AppError> {
    let now = OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        visibility: Set(options.visibility),
        state: Set(options.state),
        rules_version: Set("nommie-1.0.0".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        lock_version: Set(options.lock_version),
        ..Default::default()
    };

    let inserted = game.insert(shared.transaction()).await?;

    Ok(SnapshotGameSetup {
        game_id: inserted.id,
        lock_version: inserted.lock_version,
    })
}
