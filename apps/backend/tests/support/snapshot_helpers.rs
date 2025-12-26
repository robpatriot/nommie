// Test helpers for game snapshot integration tests
//
// Provides utilities for:
// - Creating minimal games with specific lock versions and states

use backend::db::txn::SharedTxn;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::AppError;
use sea_orm::{ActiveModelTrait, Set};
use time::OffsetDateTime;

/// Result of creating a game for snapshot testing
pub struct SnapshotGameSetup {
    pub game_id: i64,
    pub version: i32,
}

/// Options for customizing snapshot game creation
#[derive(Debug, Clone)]
pub struct SnapshotGameOptions {
    pub state: GameState,
    pub visibility: GameVisibility,
    pub version: i32,
}

impl Default for SnapshotGameOptions {
    fn default() -> Self {
        Self {
            state: GameState::Bidding,
            visibility: GameVisibility::Public,
            version: 1,
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

    pub fn with_version(mut self, version: i32) -> Self {
        self.version = version;
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
/// SnapshotGameSetup with game_id and version
///
/// # Example
/// ```
/// let options = SnapshotGameOptions::default()
///     .with_state(GameState::Bidding)
///     .with_version(5);
/// let game = create_snapshot_game(&shared, options).await?;
/// ```
///
/// **NOTE: This function bypasses the repository layer and uses ActiveModel directly.**
/// This is intentional for:
/// - Performance: Faster setup for snapshot testing
/// - Control: Full control over state, version, and other fields for snapshot edge cases
/// - Arbitrary states: Creating games in any state for snapshot validation
///
/// For simple game creation, use `repos::games::create_game()` instead.
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
        version: Set(options.version),
        ..Default::default()
    };

    let inserted = game.insert(shared.transaction()).await?;

    Ok(SnapshotGameSetup {
        game_id: inserted.id,
        version: inserted.version,
    })
}
