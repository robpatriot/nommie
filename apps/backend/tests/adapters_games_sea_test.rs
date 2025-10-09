mod common;
mod support;

use backend::adapters::games_sea::{self, GameCreate, GameUpdateState};
use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::with_txn;
use backend::entities::games::{GameState, GameVisibility};
use backend::error::AppError;
use backend::infra::state::build_state;
use rand::Rng;
use sea_orm::ConnectionTrait;
use support::db_games::delete_games_by_name;

/// GameProbe: sanitized game state for equivalence comparison.
/// Excludes id (or normalizes it) so two separate creations can be compared.
#[derive(Debug, Clone, PartialEq)]
struct GameProbe {
    join_code: String,
    state: GameState,
    visibility: GameVisibility,
    lock_version: i32,
    name: Option<String>,
    // Timestamp ordering assertions done in-flow; not compared here
}

impl GameProbe {
    /// Create a GameProbe from a game model, excluding the id for comparison
    fn from_model(model: &backend::entities::games::Model) -> Self {
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
async fn run_game_flow<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    unique_marker: &str,
) -> Result<GameProbe, AppError> {
    // 1. Create game (use unique_marker for both join_code and name for easy cleanup)
    let dto = GameCreate::new(
        unique_marker,
        None,
        Some(GameVisibility::Private),
        Some(unique_marker),
    );
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

/// Test: pooled connection vs transaction equivalence
#[tokio::test]
async fn test_games_flow_pooled_vs_txn_equivalence() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    // Run flow with REAL pooled connection (commits happen)
    // Use random 6-digit number for uniqueness
    let pooled_marker = format!("T{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let db = require_db(&state)?;
    let pooled_probe = run_game_flow(db, &pooled_marker)
        .await
        .map_err(|e| AppError::internal(format!("pooled flow failed: {e}")))?;

    // Cleanup pooled path (data was committed)
    with_txn(None, &state, |txn| {
        let pooled_marker = pooled_marker.clone();
        Box::pin(async move {
            delete_games_by_name(txn, &pooled_marker).await?;
            Ok::<_, AppError>(())
        })
    })
    .await?;

    // Run flow with transaction (auto-rollback via test policy)
    let txn_probe = with_txn(None, &state, |txn| {
        Box::pin(async move {
            let probe = run_game_flow(txn, "TEST002")
                .await
                .map_err(|e| AppError::internal(format!("txn flow failed: {e}")))?;
            Ok(probe)
        })
    })
    .await?;

    // Assert equivalence (fields should match except join_code which is different by design)
    assert_eq!(
        pooled_probe.state, txn_probe.state,
        "state should be identical"
    );
    assert_eq!(
        pooled_probe.visibility, txn_probe.visibility,
        "visibility should be identical"
    );
    assert_eq!(
        pooled_probe.lock_version, txn_probe.lock_version,
        "lock_version should be identical"
    );
    assert_eq!(
        pooled_probe.name, txn_probe.name,
        "name should be identical"
    );

    Ok(())
}

/// Test: timestamp policy consistency in both pooled and txn contexts
#[tokio::test]
async fn test_games_flow_timestamp_policy_consistent() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    // Run with REAL pooled connection (commits happen)
    // Use random 6-digit number for uniqueness
    let pooled_marker = format!("T{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let db = require_db(&state)?;
    run_game_flow(db, &pooled_marker)
        .await
        .map_err(|e| AppError::internal(format!("pooled timestamp test failed: {e}")))?;

    // Cleanup pooled path
    with_txn(None, &state, |txn| {
        let pooled_marker = pooled_marker.clone();
        Box::pin(async move {
            delete_games_by_name(txn, &pooled_marker).await?;
            Ok::<_, AppError>(())
        })
    })
    .await?;

    // Run with transaction (auto-rollback)
    with_txn(None, &state, |txn| {
        Box::pin(async move {
            run_game_flow(txn, "TEST004")
                .await
                .map_err(|e| AppError::internal(format!("txn timestamp test failed: {e}")))?;
            Ok(())
        })
    })
    .await?;

    // If we reach here, both flows passed all timestamp assertions
    Ok(())
}

/// Test: error behavior consistency (duplicate join_code constraint)
#[tokio::test]
async fn test_games_error_behavior_consistent() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    // Test with REAL pooled connection
    // Use random 6-digit number for uniqueness
    let pooled_marker = format!("D{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let db = require_db(&state)?;

    // Create first game (commits)
    let dto1 = GameCreate::new(&pooled_marker, None, None, Some("First"));
    games_sea::create_game(db, dto1)
        .await
        .map_err(|e| AppError::db(format!("create failed: {e}")))?;

    // Try to create second game with same join_code (should fail)
    let dto2 = GameCreate::new(&pooled_marker, None, None, Some("Second"));
    let pooled_result = games_sea::create_game(db, dto2).await;

    // Should have failed due to unique constraint
    assert!(
        pooled_result.is_err(),
        "pooled: duplicate join_code should fail"
    );

    // Cleanup pooled path
    with_txn(None, &state, |txn| {
        let pooled_marker = pooled_marker.clone();
        Box::pin(async move {
            delete_games_by_name(txn, &pooled_marker).await?;
            Ok::<_, AppError>(())
        })
    })
    .await?;

    // Test with transaction (auto-rollback)
    let txn_result = with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create first game
            let dto1 = GameCreate::new("DUPE002", None, None, Some("First"));
            games_sea::create_game(txn, dto1).await?;

            // Try to create second game with same join_code
            let dto2 = GameCreate::new("DUPE002", None, None, Some("Second"));
            let result = games_sea::create_game(txn, dto2).await;

            Ok::<_, AppError>(result)
        })
    })
    .await?;

    // Should have failed due to unique constraint
    assert!(txn_result.is_err(), "txn: duplicate join_code should fail");

    // Both contexts should produce errors (constraint violation)
    // The exact error type should be consistent
    Ok(())
}

/// Test: not-found errors are consistent between pooled and txn
#[tokio::test]
async fn test_games_not_found_consistent() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    let non_existent_id = 999_999_999_i64;

    // Test with REAL pooled connection
    let db = require_db(&state)?;

    let pooled_result = games_sea::find_by_id(db, non_existent_id)
        .await
        .map_err(|e| AppError::db(format!("find_by_id failed: {e}")))?;

    assert!(
        pooled_result.is_none(),
        "pooled: non-existent game should return None"
    );

    // Test with transaction
    let txn_result = with_txn(None, &state, |txn| {
        Box::pin(async move {
            let result = games_sea::find_by_id(txn, non_existent_id).await?;
            Ok::<_, AppError>(result)
        })
    })
    .await?;

    assert!(
        txn_result.is_none(),
        "txn: non-existent game should return None"
    );

    // Test with non-existent join_code (pooled)
    let pooled_join_code = games_sea::find_by_join_code(db, "NOTFOUND999")
        .await
        .map_err(|e| AppError::db(format!("find_by_join_code failed: {e}")))?;

    assert!(
        pooled_join_code.is_none(),
        "pooled: non-existent join_code should return None"
    );

    // Test with transaction
    let txn_join_code = with_txn(None, &state, |txn| {
        Box::pin(async move {
            let result = games_sea::find_by_join_code(txn, "NOTFOUND888").await?;
            Ok::<_, AppError>(result)
        })
    })
    .await?;

    assert!(
        txn_join_code.is_none(),
        "txn: non-existent join_code should return None"
    );

    Ok(())
}
