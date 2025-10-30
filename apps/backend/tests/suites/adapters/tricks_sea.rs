use backend::config::db::{DbKind, RuntimeEnv};
use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::repos::{games, rounds, tricks};

use crate::support::test_utils::short_join_code;

/// Test: create_trick and find_by_round_and_trick
#[tokio::test]
async fn test_create_trick_and_find() -> Result<(), AppError> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            // Create a trick
            let trick = tricks::create_trick(txn, round.id, 0, tricks::Suit::Hearts, 2).await?;

            assert!(trick.id > 0);
            assert_eq!(trick.round_id, round.id);
            assert_eq!(trick.trick_no, 0);
            assert_eq!(trick.lead_suit, tricks::Suit::Hearts);
            assert_eq!(trick.winner_seat, 2);

            // Find by round and trick
            let found = tricks::find_by_round_and_trick(txn, round.id, 0).await?;
            assert!(found.is_some());
            let found = found.unwrap();
            assert_eq!(found.id, trick.id);
            assert_eq!(found.lead_suit, tricks::Suit::Hearts);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_all_by_round returns tricks in order
#[tokio::test]
async fn test_find_all_by_round_ordered() -> Result<(), AppError> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 3, 0).await?;

            // Create tricks out of order
            tricks::create_trick(txn, round.id, 2, tricks::Suit::Clubs, 0).await?;
            tricks::create_trick(txn, round.id, 0, tricks::Suit::Hearts, 1).await?;
            tricks::create_trick(txn, round.id, 1, tricks::Suit::Spades, 2).await?;

            // Find all should return ordered by trick_no
            let all_tricks = tricks::find_all_by_round(txn, round.id).await?;
            assert_eq!(all_tricks.len(), 3);
            assert_eq!(all_tricks[0].trick_no, 0);
            assert_eq!(all_tricks[1].trick_no, 1);
            assert_eq!(all_tricks[2].trick_no, 2);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: count_tricks_by_round
#[tokio::test]
async fn test_count_tricks() -> Result<(), AppError> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 5, 0).await?;

            // Initially 0 tricks
            let count = tricks::count_tricks_by_round(txn, round.id).await?;
            assert_eq!(count, 0);

            // Add tricks
            tricks::create_trick(txn, round.id, 0, tricks::Suit::Hearts, 0).await?;
            tricks::create_trick(txn, round.id, 1, tricks::Suit::Spades, 1).await?;

            let count = tricks::count_tricks_by_round(txn, round.id).await?;
            assert_eq!(count, 2);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: unique constraint on (round_id, trick_no)
#[tokio::test]
async fn test_unique_constraint_round_trick() -> Result<(), AppError> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            // Create first trick
            tricks::create_trick(txn, round.id, 0, tricks::Suit::Hearts, 0).await?;

            // Try to create duplicate trick with same trick_no
            let result = tricks::create_trick(txn, round.id, 0, tricks::Suit::Spades, 1).await;

            assert!(result.is_err(), "Duplicate trick should fail");

            // Verify it's a conflict error
            match result.unwrap_err() {
                backend::errors::domain::DomainError::Conflict(_, _) => {
                    // Expected
                }
                e => panic!("Expected Conflict error, got {e:?}"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: all four suits can be used as lead
#[tokio::test]
async fn test_all_suits_as_lead() -> Result<(), AppError> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 4, 0).await?;

            // Create tricks with different lead suits
            tricks::create_trick(txn, round.id, 0, tricks::Suit::Clubs, 0).await?;
            tricks::create_trick(txn, round.id, 1, tricks::Suit::Diamonds, 1).await?;
            tricks::create_trick(txn, round.id, 2, tricks::Suit::Hearts, 2).await?;
            tricks::create_trick(txn, round.id, 3, tricks::Suit::Spades, 3).await?;

            let all_tricks = tricks::find_all_by_round(txn, round.id).await?;
            assert_eq!(all_tricks.len(), 4);
            assert_eq!(all_tricks[0].lead_suit, tricks::Suit::Clubs);
            assert_eq!(all_tricks[1].lead_suit, tricks::Suit::Diamonds);
            assert_eq!(all_tricks[2].lead_suit, tricks::Suit::Hearts);
            assert_eq!(all_tricks[3].lead_suit, tricks::Suit::Spades);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
