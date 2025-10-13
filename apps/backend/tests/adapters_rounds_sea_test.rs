mod common;
mod support;

use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::repos::{games, rounds};
use support::test_utils::short_join_code;

/// Test: create_round and find_by_id roundtrip
#[tokio::test]
async fn test_create_round_and_find_by_id_roundtrip() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create a game first
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;

            // Create a round
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            assert!(round.id > 0, "Round ID should be positive");
            assert_eq!(round.game_id, game.id);
            assert_eq!(round.round_no, 1);
            assert_eq!(round.hand_size, 13);
            assert_eq!(round.dealer_pos, 0);
            assert_eq!(round.trump, None);
            assert_eq!(round.completed_at, None);

            // Find by ID
            let found = rounds::find_by_id(txn, round.id).await?;
            assert!(found.is_some(), "Round should be found");
            let found = found.unwrap();
            assert_eq!(found.id, round.id);
            assert_eq!(found.game_id, round.game_id);
            assert_eq!(found.round_no, round.round_no);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_by_game_and_round locates correct round
#[tokio::test]
async fn test_find_by_game_and_round() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create a game
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;

            // Create multiple rounds
            let round1 = rounds::create_round(txn, game.id, 1, 13, 0).await?;
            let round2 = rounds::create_round(txn, game.id, 2, 12, 1).await?;

            // Find specific round
            let found = rounds::find_by_game_and_round(txn, game.id, 2).await?;
            assert!(found.is_some(), "Round 2 should be found");
            let found = found.unwrap();
            assert_eq!(found.id, round2.id);
            assert_eq!(found.round_no, 2);
            assert_eq!(found.hand_size, 12);

            // Verify round 1 is different
            let found1 = rounds::find_by_game_and_round(txn, game.id, 1).await?;
            assert!(found1.is_some());
            let found1 = found1.unwrap();
            assert_eq!(found1.id, round1.id);
            assert_eq!(found1.round_no, 1);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_by_game_and_round returns None for non-existent round
#[tokio::test]
async fn test_find_by_game_and_round_not_found() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create a game
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;

            // Try to find non-existent round
            let found = rounds::find_by_game_and_round(txn, game.id, 99).await?;
            assert!(found.is_none(), "Non-existent round should return None");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: update_trump sets trump selection
#[tokio::test]
async fn test_update_trump() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create game and round
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            // Initially no trump
            assert_eq!(round.trump, None);

            // Set trump
            let updated = rounds::update_trump(txn, round.id, rounds::Trump::Hearts).await?;
            assert_eq!(updated.trump, Some(rounds::Trump::Hearts));
            assert_eq!(updated.id, round.id);

            // Verify it persisted
            let found = rounds::find_by_id(txn, round.id).await?;
            assert!(found.is_some());
            assert_eq!(found.unwrap().trump, Some(rounds::Trump::Hearts));

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: update_trump with NoTrump
#[tokio::test]
async fn test_update_trump_no_trump() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            let updated = rounds::update_trump(txn, round.id, rounds::Trump::NoTrump).await?;
            assert_eq!(updated.trump, Some(rounds::Trump::NoTrump));

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: complete_round sets completed_at timestamp
#[tokio::test]
async fn test_complete_round() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            // Initially not completed
            assert_eq!(round.completed_at, None);

            // Complete the round
            let updated = rounds::complete_round(txn, round.id).await?;
            assert!(updated.completed_at.is_some(), "completed_at should be set");
            assert_eq!(updated.id, round.id);

            // Verify it persisted
            let found = rounds::find_by_id(txn, round.id).await?;
            assert!(found.is_some());
            assert!(found.unwrap().completed_at.is_some());

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: unique constraint on (game_id, round_no)
#[tokio::test]
async fn test_unique_constraint_game_round() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;

            // Create first round
            rounds::create_round(txn, game.id, 1, 13, 0).await?;

            // Try to create duplicate round with same round_no
            let result = rounds::create_round(txn, game.id, 1, 12, 1).await;

            assert!(result.is_err(), "Duplicate round should fail");

            // Verify it's a unique violation
            match result.unwrap_err() {
                backend::errors::domain::DomainError::Conflict(kind, _) => {
                    assert!(
                        matches!(kind, backend::errors::domain::ConflictKind::Other(_)),
                        "Expected Conflict(Other), got {kind:?}"
                    );
                }
                e => panic!("Expected Conflict error, got {e:?}"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
