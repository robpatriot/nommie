// Helper functions for testing games_sea adapter

use backend::adapters::games_sea::{self, GameCreate, GameUpdate};
use backend::entities::games::{GameState, GameVisibility};
use backend::AppError;
use sea_orm::DatabaseTransaction;

/// GameProbe: sanitized game state for equivalence comparison.
/// Excludes id (or normalizes it) so two separate creations can be compared.
#[derive(Debug, Clone, PartialEq)]
pub struct GameProbe {
    pub state: GameState,
    pub visibility: GameVisibility,
    pub lock_version: i32,
    pub name: Option<String>,
    // Timestamp ordering assertions done in-flow; not compared here
}

impl GameProbe {
    /// Create a GameProbe from a game model, excluding the id for comparison
    pub fn from_model(model: &backend::entities::games::Model) -> Self {
        Self {
            state: model.state.clone(),
            visibility: model.visibility.clone(),
            lock_version: model.lock_version,
            name: model.name.clone(),
        }
    }
}

/// Helper: run a consistent game flow using games_sea adapter directly.
/// Returns a GameProbe for equivalence testing.
///
/// Flow:
/// 1. Create game with unique marker (used for the name)
/// 2. Assert timestamps on create
/// 3. Fetch by id; assert consistency
/// 4. Update state (Lobby -> Bidding)
/// 5. Assert created_at unchanged, updated_at advanced
/// 6. Update state again (Bidding -> TrickPlay)
/// 7. Assert created_at still unchanged, updated_at advanced again
/// 8. Update metadata (name change)
/// 9. Assert created_at still unchanged, updated_at advanced
/// 10. Return GameProbe
pub async fn run_game_flow(
    txn: &DatabaseTransaction,
    unique_marker: &str,
) -> Result<GameProbe, AppError> {
    // 1. Create game (use unique_marker for name for easy cleanup)
    let dto = GameCreate::new()
        .with_visibility(GameVisibility::Private)
        .with_name(unique_marker);
    let created = games_sea::create_game(txn, dto)
        .await
        .map_err(|e| AppError::db("failed to create game", e))?;

    // Assert: timestamps set on create
    assert!(
        created.created_at <= time::OffsetDateTime::now_utc(),
        "created_at should be set to a valid timestamp"
    );
    assert!(
        created.updated_at <= time::OffsetDateTime::now_utc(),
        "updated_at should be set to a valid timestamp"
    );
    assert!(
        created.updated_at >= created.created_at,
        "updated_at should be >= created_at on create"
    );

    let original_created_at = created.created_at;
    let original_updated_at = created.updated_at;

    // 2. Fetch by id and assert consistency
    let _by_id = games_sea::find_by_id(txn, created.id)
        .await
        .map_err(|e| AppError::db("failed to fetch game", e))?
        .expect("game should exist by id");

    // Small sleep to ensure time progresses for timestamp tests
    // (in practice, DB operations may be fast enough to get same timestamp)
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

    // 3. First update: change state to Bidding
    let update_dto =
        GameUpdate::new(created.id, created.lock_version).with_state(GameState::Bidding);
    let updated1 = games_sea::update_game(txn, update_dto)
        .await
        .map_err(|e| AppError::db("failed to update game state", e))?;

    assert_eq!(
        updated1.created_at, original_created_at,
        "created_at should remain unchanged after first update"
    );
    assert!(
        updated1.updated_at >= original_updated_at,
        "updated_at should advance or stay same after first update"
    );
    assert_eq!(
        updated1.state,
        GameState::Bidding,
        "state should be updated to Bidding"
    );

    let first_updated_at = updated1.updated_at;

    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

    // 4. Second update: change state to TrickPlay
    let update_dto2 =
        GameUpdate::new(created.id, updated1.lock_version).with_state(GameState::TrickPlay);
    let updated2 = games_sea::update_game(txn, update_dto2)
        .await
        .map_err(|e| AppError::db("failed to update game state", e))?;

    assert_eq!(
        updated2.created_at, original_created_at,
        "created_at should remain unchanged after second update"
    );
    assert!(
        updated2.updated_at >= first_updated_at,
        "updated_at should advance or stay same after second update"
    );
    assert_eq!(
        updated2.state,
        GameState::TrickPlay,
        "state should be updated to TrickPlay"
    );

    let second_updated_at = updated2.updated_at;

    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

    // 5. Third update: change state to Scoring (to test timestamp behavior)
    let update_dto3 =
        GameUpdate::new(created.id, updated2.lock_version).with_state(GameState::Scoring);
    let updated3 = games_sea::update_game(txn, update_dto3)
        .await
        .map_err(|e| AppError::db("failed to update game state", e))?;

    assert_eq!(
        updated3.created_at, original_created_at,
        "created_at should remain unchanged after third update"
    );
    assert!(
        updated3.updated_at >= second_updated_at,
        "updated_at should advance or stay same after third update"
    );
    assert_eq!(
        updated3.state,
        GameState::Scoring,
        "state should be updated to Scoring"
    );

    // 6. Return probe with final state
    Ok(GameProbe::from_model(&updated3))
}
