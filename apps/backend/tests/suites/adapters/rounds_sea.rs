use backend::adapters::games_sea::GameCreate;
use backend::db::txn::with_txn;
use backend::domain::Trump;
use backend::repos::{games, rounds};
use backend::utils::join_code::generate_join_code;
use backend::AppError;

use crate::support::build_test_state;

/// Test: create_round and find_by_id roundtrip
#[tokio::test]
async fn test_create_round_and_find_by_id_roundtrip() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;

            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            assert!(round.id > 0, "Round ID should be positive");
            assert_eq!(round.game_id, game.id);
            assert_eq!(round.round_no, 1);
            assert_eq!(round.hand_size, 13);
            assert_eq!(round.dealer_pos, 0);
            assert_eq!(round.trump, None);
            assert_eq!(round.completed_at, None);

            let found = rounds::find_by_game_and_round(txn, game.id, 1).await?;
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
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;

            let round1 = rounds::create_round(txn, game.id, 1, 13, 0).await?;
            let round2 = rounds::create_round(txn, game.id, 2, 12, 1).await?;

            let found = rounds::find_by_game_and_round(txn, game.id, 2).await?;
            assert!(found.is_some(), "Round 2 should be found");
            let found = found.unwrap();
            assert_eq!(found.id, round2.id);
            assert_eq!(found.round_no, 2);
            assert_eq!(found.hand_size, 12);

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
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;

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
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            assert_eq!(round.trump, None);

            let updated = rounds::update_trump(txn, round.id, Trump::Hearts).await?;
            assert_eq!(updated.trump, Some(Trump::Hearts));
            assert_eq!(updated.id, round.id);

            let found = rounds::find_by_game_and_round(txn, game.id, 1).await?;
            assert!(found.is_some());
            assert_eq!(found.unwrap().trump, Some(Trump::Hearts));

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: update_trump with NoTrumps
#[tokio::test]
async fn test_update_trump_no_trump() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            let updated = rounds::update_trump(txn, round.id, Trump::NoTrumps).await?;
            assert_eq!(updated.trump, Some(Trump::NoTrumps));

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: complete_round sets completed_at timestamp
#[tokio::test]
async fn test_complete_round() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            assert_eq!(round.completed_at, None);

            let updated = rounds::complete_round(txn, round.id).await?;
            assert!(updated.completed_at.is_some(), "completed_at should be set");
            assert_eq!(updated.id, round.id);

            let found = rounds::find_by_game_and_round(txn, game.id, 1).await?;
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
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;

            rounds::create_round(txn, game.id, 1, 13, 0).await?;

            let result = rounds::create_round(txn, game.id, 1, 12, 1).await;

            assert!(result.is_err(), "Duplicate round should fail");

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
