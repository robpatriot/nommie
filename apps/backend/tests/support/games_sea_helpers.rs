//! Helper functions for testing games_sea adapter

use backend::adapters::games_sea::{self, GameCreate, GameUpdateState};
use backend::entities::games::{GameState, GameVisibility};
use backend::error::AppError;
use sea_orm::ConnectionTrait;

/// GameProbe: sanitized game state for equivalence comparison.
/// Excludes id (or normalizes it) so two separate creations can be compared.
#[derive(Debug, Clone, PartialEq)]
pub struct GameProbe {
    pub join_code: String,
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
            join_code: model.join_code.clone().unwrap_or_default(),
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
/// 1. Create game with unique marker (used for both join_code and name)
/// 2. Assert timestamps on create
/// 3. Fetch by id and by join_code; assert consistency
/// 4. Update state (Lobby -> Bidding)
/// 5. Assert created_at unchanged, updated_at advanced
/// 6. Update state again (Bidding -> TrickPlay)
/// 7. Assert created_at still unchanged, updated_at advanced again
/// 8. Update metadata (name change)
/// 9. Assert created_at still unchanged, updated_at advanced
/// 10. Return GameProbe
pub async fn run_game_flow<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    unique_marker: &str,
) -> Result<GameProbe, AppError> {
    // 1. Create game (use unique_marker for both join_code and name for easy cleanup)
    let dto = GameCreate::new(unique_marker)
        .with_visibility(GameVisibility::Private)
        .with_name(unique_marker);
    let created = games_sea::create_game(conn, dto)
        .await
        .map_err(|e| AppError::db(format!("create_game failed: {e}")))?;

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

    // 2. Fetch by id and by join_code; assert consistency
    let by_id = games_sea::find_by_id(conn, created.id)
        .await
        .map_err(|e| AppError::db(format!("find_by_id failed: {e}")))?
        .expect("game should exist by id");
    let by_join_code = games_sea::find_by_join_code(conn, unique_marker)
        .await
        .map_err(|e| AppError::db(format!("find_by_join_code failed: {e}")))?
        .expect("game should exist by join_code");

    assert_eq!(
        by_id.id, by_join_code.id,
        "id should match when fetched by id vs join_code"
    );
    assert_eq!(
        by_id.join_code, by_join_code.join_code,
        "join_code should match"
    );

    // Small sleep to ensure time progresses for timestamp tests
    // (in practice, DB operations may be fast enough to get same timestamp)
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

    // 3. First update: change state to Bidding
    let update_dto = GameUpdateState::new(created.id, GameState::Bidding);
    let updated1 = games_sea::update_state(conn, update_dto)
        .await
        .map_err(|e| AppError::db(format!("update_state failed: {e}")))?;

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
    let update_dto2 = GameUpdateState::new(created.id, GameState::TrickPlay);
    let updated2 = games_sea::update_state(conn, update_dto2)
        .await
        .map_err(|e| AppError::db(format!("update_state failed: {e}")))?;

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

    // 5. Third update: change metadata (name)
    let update_meta =
        games_sea::GameUpdateMetadata::new(created.id, Some("UpdatedName"), updated2.visibility);
    let updated3 = games_sea::update_metadata(conn, update_meta)
        .await
        .map_err(|e| AppError::db(format!("update_metadata failed: {e}")))?;

    assert_eq!(
        updated3.created_at, original_created_at,
        "created_at should remain unchanged after metadata update"
    );
    assert!(
        updated3.updated_at >= second_updated_at,
        "updated_at should advance or stay same after metadata update"
    );
    assert_eq!(
        updated3.name,
        Some("UpdatedName".to_string()),
        "name should be updated"
    );

    // 6. Return probe with final state
    Ok(GameProbe::from_model(&updated3))
}
