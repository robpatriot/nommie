// Repository-level tests for optimistic locking negative paths.
//
// These tests verify that the adapter correctly handles:
// 1. Attempting to update a non-existent game → NotFound
// 2. Attempting to update with a stale lock_version → OptimisticLock conflict
//
// All tests use transactions with automatic rollback via the test policy.

use backend::adapters::games_sea::{self, GameCreate, GameUpdate};
use backend::db::txn::with_txn;
use backend::entities::games::GameState;
use backend::AppError;
use backend::errors::domain::{ConflictKind, DomainError, NotFoundKind};

use crate::support::build_test_state;

/// Test that attempting to update a non-existent game returns NotFound.
#[tokio::test]
async fn test_update_nonexistent_game_returns_not_found() -> Result<(), AppError> {
    let state = build_test_state().await?;

    let result = with_txn(None, &state, |txn| {
        Box::pin(async move {
            let non_existent_id = 999_999_999_i64;
            let expected_lock_version = 0;

            // Attempt to update a game that doesn't exist
            let update_dto =
                GameUpdate::new(non_existent_id, expected_lock_version)
                    .with_state(GameState::Bidding);
            let result = games_sea::update_game(txn, update_dto).await;

            // Should return an error
            assert!(result.is_err(), "update should fail for non-existent game");

            // Convert to DomainError to verify error type
            let db_err = result.unwrap_err();
            let domain_err: DomainError = db_err.into();

            // Should be a NotFound error
            match domain_err {
                DomainError::NotFound(kind, detail) => {
                    // Verify it's the correct kind of NotFound
                    assert!(
                        matches!(kind, NotFoundKind::Game),
                        "should be NotFound variant with NotFoundKind::Game"
                    );
                    assert!(
                        detail.contains("not found") || detail.contains("Not found"),
                        "detail should mention 'not found', got: {detail}"
                    );
                }
                other => panic!("expected NotFound, got: {other:?}"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await;

    result
}

/// Test that attempting to update with a stale lock_version returns an OptimisticLock conflict
/// with the correct expected and actual version numbers.
#[tokio::test]
async fn test_optimistic_lock_conflict_returns_expected_and_actual() -> Result<(), AppError> {
    let state = build_test_state().await?;

    let result = with_txn(None, &state, |txn| {
        Box::pin(async move {
            // 1. Create a game (initial lock_version should be 1)
            let create_dto = GameCreate::new("LOCK001").with_name("OptimisticLockTest");
            let game = games_sea::create_game(txn, create_dto)
                .await
                .map_err(|e| AppError::db("failed to create game", e))?;

            let initial_lock_version = game.lock_version;
            assert_eq!(initial_lock_version, 1, "initial lock_version should be 1");

            // 2. First update: succeed with correct lock_version
            let update1 = GameUpdate::new(game.id, initial_lock_version)
                .with_state(GameState::Bidding);
            let game_after_update1 = games_sea::update_game(txn, update1)
                .await
                .map_err(|e| AppError::db("failed to update game state", e))?;

            assert_eq!(
                game_after_update1.lock_version,
                initial_lock_version + 1,
                "lock_version should increment to 2"
            );
            assert_eq!(
                game_after_update1.state,
                GameState::Bidding,
                "state should be updated to Bidding"
            );

            // 3. Second update: attempt with stale lock_version (use the original version 1)
            let stale_version = initial_lock_version; // Still 1, but actual is now 2
            let update2 = GameUpdate::new(game.id, stale_version)
                .with_state(GameState::TrickPlay);
            let result = games_sea::update_game(txn, update2).await;

            // Should fail with optimistic lock conflict
            assert!(
                result.is_err(),
                "update with stale lock_version should fail"
            );

            // Convert to DomainError to verify error type and detail
            let db_err = result.unwrap_err();
            let domain_err: DomainError = db_err.into();

            // Should be a Conflict with OptimisticLock kind
            match domain_err {
                DomainError::Conflict(kind, detail) => {
                    // Verify it's OptimisticLock
                    assert!(
                        matches!(kind, ConflictKind::OptimisticLock),
                        "should be OptimisticLock conflict, got: {kind:?}"
                    );

                    // Verify detail contains version information
                    assert!(
                        detail.contains("expected version"),
                        "detail should mention 'expected version', got: {detail}"
                    );
                    assert!(
                        detail.contains("actual version"),
                        "detail should mention 'actual version', got: {detail}"
                    );

                    // Verify the actual numbers are present
                    assert!(
                        detail.contains(&format!("expected version {stale_version}")),
                        "detail should contain expected version {stale_version}, got: {detail}"
                    );
                    assert!(
                        detail.contains(&format!("actual version {}", initial_lock_version + 1)),
                        "detail should contain actual version {}, got: {detail}",
                        initial_lock_version + 1
                    );
                }
                other => panic!("expected Conflict(OptimisticLock), got: {other:?}"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await;

    result
}

/// Additional test: verify that multiple concurrent stale updates all fail appropriately
#[tokio::test]
async fn test_multiple_stale_updates_all_fail() -> Result<(), AppError> {
    let state = build_test_state().await?;

    let result = with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create a game
            let create_dto = GameCreate::new("LOCK002").with_name("MultipleStaleTest");
            let game = games_sea::create_game(txn, create_dto)
                .await
                .map_err(|e| AppError::db("failed to create game", e))?;

            let initial_version = game.lock_version;

            // Successfully update once
            let update1 = GameUpdate::new(game.id, initial_version)
                .with_state(GameState::Bidding);
            let game_v2 = games_sea::update_game(txn, update1)
                .await
                .map_err(|e| AppError::db("failed to update game state", e))?;

            assert_eq!(game_v2.lock_version, initial_version + 1);

            // Successfully update again
            let update2 = GameUpdate::new(game.id, game_v2.lock_version)
                .with_state(GameState::TrickPlay);
            let game_v3 = games_sea::update_game(txn, update2)
                .await
                .map_err(|e| AppError::db("failed to update game state", e))?;

            assert_eq!(game_v3.lock_version, initial_version + 2);

            // Now try multiple stale updates - all should fail
            // Try with version 1 (actual is 3)
            let stale_update_1 = GameUpdate::new(game.id, initial_version)
                .with_state(GameState::Scoring);
            let result1 = games_sea::update_game(txn, stale_update_1).await;
            assert!(result1.is_err(), "stale update with v1 should fail");

            // Try with version 2 (actual is still 3)
            let stale_update_2 = GameUpdate::new(game.id, initial_version + 1)
                .with_state(GameState::Scoring);
            let result2 = games_sea::update_game(txn, stale_update_2).await;
            assert!(result2.is_err(), "stale update with v2 should fail");

            // Verify both are optimistic lock conflicts
            let err1: DomainError = result1.unwrap_err().into();
            let err2: DomainError = result2.unwrap_err().into();

            assert!(
                matches!(err1, DomainError::Conflict(ConflictKind::OptimisticLock, _)),
                "first stale update should be OptimisticLock conflict"
            );
            assert!(
                matches!(err2, DomainError::Conflict(ConflictKind::OptimisticLock, _)),
                "second stale update should be OptimisticLock conflict"
            );

            Ok::<_, AppError>(())
        })
    })
    .await;

    result
}
