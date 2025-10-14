//! Adapter tests for games_sea - CRUD operations, constraints, and invariants.

use backend::adapters::games_sea::{
    self, GameCreate, GameUpdateMetadata, GameUpdateRound, GameUpdateState,
};
use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::entities::games::{GameState, GameVisibility};
use backend::error::AppError;
use backend::errors::domain::{ConflictKind, DomainError, NotFoundKind};
use backend::infra::state::build_state;

use crate::support::test_utils::short_join_code as unique_join_code;

/// Test: create_game and find_by_id
#[tokio::test]
async fn test_create_and_find_by_id() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();
            let dto = GameCreate::new(&join_code)
                .with_visibility(GameVisibility::Private)
                .with_name("Test Game");

            let created = games_sea::create_game(txn, dto).await?;

            assert!(created.id > 0);
            assert_eq!(created.join_code, Some(join_code.clone()));
            assert_eq!(created.state, GameState::Lobby);
            assert_eq!(created.visibility, GameVisibility::Private);
            assert_eq!(created.name, Some("Test Game".to_string()));
            assert_eq!(created.lock_version, 1);

            // Find by id
            let found = games_sea::find_by_id(txn, created.id).await?;
            assert!(found.is_some());
            let found = found.unwrap();
            assert_eq!(found.id, created.id);
            assert_eq!(found.join_code, created.join_code);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_by_join_code
#[tokio::test]
async fn test_find_by_join_code() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();
            let dto = GameCreate::new(&join_code);
            let created = games_sea::create_game(txn, dto).await?;

            // Find by join_code
            let found = games_sea::find_by_join_code(txn, &join_code).await?;
            assert!(found.is_some());
            let found = found.unwrap();
            assert_eq!(found.id, created.id);
            assert_eq!(found.join_code, Some(join_code));

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_by_id returns None for non-existent game
#[tokio::test]
async fn test_find_by_id_not_found() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let non_existent_id = 999_999_999_i64;
            let result = games_sea::find_by_id(txn, non_existent_id).await?;
            assert!(result.is_none());

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_by_join_code returns None for non-existent code
#[tokio::test]
async fn test_find_by_join_code_not_found() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let result = games_sea::find_by_join_code(txn, "NOTFOUND999").await?;
            assert!(result.is_none());

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: require_game returns game if exists
#[tokio::test]
async fn test_require_game_exists() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();
            let created = games_sea::create_game(txn, GameCreate::new(&join_code)).await?;

            // require_game should succeed
            let required = games_sea::require_game(txn, created.id).await?;
            assert_eq!(required.id, created.id);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: require_game returns error if not found
#[tokio::test]
async fn test_require_game_not_found() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let non_existent_id = 999_999_999_i64;
            let result = games_sea::require_game(txn, non_existent_id).await;

            assert!(result.is_err());
            let err: DomainError = result.unwrap_err().into();
            assert!(
                matches!(err, DomainError::NotFound(NotFoundKind::Other(_), _)),
                "should be NotFound error"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: duplicate join_code constraint violation
#[tokio::test]
async fn test_duplicate_join_code_fails() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();

            // Create first game
            games_sea::create_game(txn, GameCreate::new(&join_code)).await?;

            // Try to create second game with same join_code
            let result = games_sea::create_game(txn, GameCreate::new(&join_code)).await;

            assert!(result.is_err(), "duplicate join_code should fail");
            let err: DomainError = result.unwrap_err().into();
            assert!(
                matches!(
                    err,
                    DomainError::Conflict(ConflictKind::JoinCodeConflict, _)
                ),
                "should be JoinCodeConflict, got: {err:?}"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: update_metadata changes name and visibility
#[tokio::test]
async fn test_update_metadata() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();
            let game = games_sea::create_game(
                txn,
                GameCreate::new(&join_code)
                    .with_name("Original")
                    .with_visibility(GameVisibility::Private),
            )
            .await?;

            assert_eq!(game.name, Some("Original".to_string()));
            assert_eq!(game.visibility, GameVisibility::Private);
            let original_lock = game.lock_version;

            // Update metadata
            let update = GameUpdateMetadata::new(
                game.id,
                Some("Updated"),
                GameVisibility::Public,
                original_lock,
            );
            let updated = games_sea::update_metadata(txn, update).await?;

            assert_eq!(updated.name, Some("Updated".to_string()));
            assert_eq!(updated.visibility, GameVisibility::Public);
            assert_eq!(updated.lock_version, original_lock + 1);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: update_round changes round fields
#[tokio::test]
async fn test_update_round() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();
            let game = games_sea::create_game(txn, GameCreate::new(&join_code)).await?;

            assert!(game.current_round.is_none());
            assert!(game.starting_dealer_pos.is_none());
            assert_eq!(game.current_trick_no, 0);

            // Update round fields
            let update = GameUpdateRound::new(game.id, game.lock_version)
                .with_current_round(1)
                .with_starting_dealer_pos(2)
                .with_current_trick_no(3);

            let updated = games_sea::update_round(txn, update).await?;

            assert_eq!(updated.current_round, Some(1));
            assert_eq!(updated.starting_dealer_pos, Some(2));
            assert_eq!(updated.current_trick_no, 3);
            assert_eq!(updated.lock_version, game.lock_version + 1);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: timestamp invariants on create
#[tokio::test]
async fn test_create_timestamps() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();
            let game = games_sea::create_game(txn, GameCreate::new(&join_code)).await?;

            let now = time::OffsetDateTime::now_utc();

            // created_at should be set and reasonable
            assert!(
                game.created_at <= now,
                "created_at should be <= current time"
            );
            assert!(
                game.created_at > now - time::Duration::seconds(5),
                "created_at should be recent"
            );

            // updated_at should equal created_at on create
            assert_eq!(
                game.updated_at, game.created_at,
                "updated_at should equal created_at on create"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: timestamp invariants on update
#[tokio::test]
async fn test_update_timestamps() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();
            let game = games_sea::create_game(txn, GameCreate::new(&join_code)).await?;

            let original_created_at = game.created_at;
            let original_updated_at = game.updated_at;

            // Small delay to ensure time can advance
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

            // Update state
            let update = GameUpdateState::new(game.id, GameState::Bidding, game.lock_version);
            let updated = games_sea::update_state(txn, update).await?;

            // created_at must remain constant
            assert_eq!(
                updated.created_at, original_created_at,
                "created_at must remain constant"
            );

            // updated_at should advance (or stay same if time didn't progress)
            assert!(
                updated.updated_at >= original_updated_at,
                "updated_at should advance or stay same"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: lock_version increments on each update
#[tokio::test]
async fn test_lock_version_increments() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();
            let game = games_sea::create_game(txn, GameCreate::new(&join_code)).await?;

            assert_eq!(game.lock_version, 1, "initial lock_version should be 1");

            // First update
            let update1 = GameUpdateState::new(game.id, GameState::Bidding, game.lock_version);
            let after1 = games_sea::update_state(txn, update1).await?;
            assert_eq!(after1.lock_version, 2);

            // Second update
            let update2 =
                GameUpdateState::new(after1.id, GameState::TrickPlay, after1.lock_version);
            let after2 = games_sea::update_state(txn, update2).await?;
            assert_eq!(after2.lock_version, 3);

            // update_metadata should also increment
            let update_meta = GameUpdateMetadata::new(
                after2.id,
                Some("New Name"),
                after2.visibility,
                after2.lock_version,
            );
            let after3 = games_sea::update_metadata(txn, update_meta).await?;
            assert_eq!(after3.lock_version, 4);

            // update_round should also increment
            let update_round =
                GameUpdateRound::new(after3.id, after3.lock_version).with_current_round(1);
            let after4 = games_sea::update_round(txn, update_round).await?;
            assert_eq!(after4.lock_version, 5);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: update_metadata with None name clears the name
#[tokio::test]
async fn test_update_metadata_clear_name() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();
            let game =
                games_sea::create_game(txn, GameCreate::new(&join_code).with_name("Has Name"))
                    .await?;

            assert_eq!(game.name, Some("Has Name".to_string()));

            // Clear name by passing None
            let update = GameUpdateMetadata::new(
                game.id,
                None::<String>,
                game.visibility,
                game.lock_version,
            );
            let updated = games_sea::update_metadata(txn, update).await?;

            assert_eq!(updated.name, None);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: update_round with partial fields only updates specified fields
#[tokio::test]
async fn test_update_round_partial() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();
            let game = games_sea::create_game(txn, GameCreate::new(&join_code)).await?;

            // Update only current_round
            let update1 = GameUpdateRound::new(game.id, game.lock_version).with_current_round(5);
            let updated1 = games_sea::update_round(txn, update1).await?;

            assert_eq!(updated1.current_round, Some(5));
            assert_eq!(updated1.starting_dealer_pos, None);
            assert_eq!(updated1.current_trick_no, 0);

            // Now update only starting_dealer_pos
            let update2 = GameUpdateRound::new(updated1.id, updated1.lock_version)
                .with_starting_dealer_pos(3);
            let updated2 = games_sea::update_round(txn, update2).await?;

            assert_eq!(updated2.current_round, Some(5)); // Unchanged
            assert_eq!(updated2.starting_dealer_pos, Some(3)); // Updated
            assert_eq!(updated2.current_trick_no, 0); // Unchanged

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: update_state transitions work correctly
#[tokio::test]
async fn test_update_state_transitions() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();
            let game = games_sea::create_game(txn, GameCreate::new(&join_code)).await?;

            assert_eq!(game.state, GameState::Lobby);

            // Lobby -> Bidding
            let update1 = GameUpdateState::new(game.id, GameState::Bidding, game.lock_version);
            let after1 = games_sea::update_state(txn, update1).await?;
            assert_eq!(after1.state, GameState::Bidding);

            // Bidding -> TrumpSelection
            let update2 =
                GameUpdateState::new(after1.id, GameState::TrumpSelection, after1.lock_version);
            let after2 = games_sea::update_state(txn, update2).await?;
            assert_eq!(after2.state, GameState::TrumpSelection);

            // TrumpSelection -> TrickPlay
            let update3 =
                GameUpdateState::new(after2.id, GameState::TrickPlay, after2.lock_version);
            let after3 = games_sea::update_state(txn, update3).await?;
            assert_eq!(after3.state, GameState::TrickPlay);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: create_game with minimal DTO uses defaults
#[tokio::test]
async fn test_create_game_defaults() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = unique_join_code();
            let game = games_sea::create_game(txn, GameCreate::new(&join_code)).await?;

            // Check defaults
            assert_eq!(game.visibility, GameVisibility::Private);
            assert_eq!(game.state, GameState::Lobby);
            assert_eq!(game.name, None);
            assert_eq!(game.created_by, None);
            assert_eq!(game.rng_seed, None);
            assert_eq!(game.current_round, None);
            assert_eq!(game.starting_dealer_pos, None);
            assert_eq!(game.current_trick_no, 0);
            assert_eq!(game.lock_version, 1);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
