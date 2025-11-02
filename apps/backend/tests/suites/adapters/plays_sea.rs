use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::repos::{games, plays, rounds, tricks};

use crate::support::build_test_state;
use crate::support::test_utils::short_join_code;

/// Test: create_play and find_all_by_trick
#[tokio::test]
async fn test_create_play_and_find_all() -> Result<(), AppError> {
    let state = build_test_state().await.expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 3, 0).await?;
            let trick = tricks::create_trick(txn, round.id, 0, tricks::Suit::Hearts, 0).await?;

            // Create plays
            let card1 = plays::Card {
                suit: "HEARTS".into(),
                rank: "ACE".into(),
            };
            let card2 = plays::Card {
                suit: "HEARTS".into(),
                rank: "KING".into(),
            };

            let play1 = plays::create_play(txn, trick.id, 0, card1.clone(), 0).await?;
            let _play2 = plays::create_play(txn, trick.id, 1, card2.clone(), 1).await?;

            assert!(play1.id > 0);
            assert_eq!(play1.player_seat, 0);
            assert_eq!(play1.card, card1);
            assert_eq!(play1.play_order, 0);

            // Find all plays (should be ordered)
            let all_plays = plays::find_all_by_trick(txn, trick.id).await?;
            assert_eq!(all_plays.len(), 2);
            assert_eq!(all_plays[0].play_order, 0);
            assert_eq!(all_plays[1].play_order, 1);
            assert_eq!(all_plays[0].card, card1);
            assert_eq!(all_plays[1].card, card2);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: count_plays_by_trick
#[tokio::test]
async fn test_count_plays() -> Result<(), AppError> {
    let state = build_test_state().await.expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 4, 0).await?;
            let trick = tricks::create_trick(txn, round.id, 0, tricks::Suit::Hearts, 0).await?;

            // Initially 0 plays
            let count = plays::count_plays_by_trick(txn, trick.id).await?;
            assert_eq!(count, 0);

            // Add plays
            let card = plays::Card {
                suit: "HEARTS".into(),
                rank: "ACE".into(),
            };
            plays::create_play(txn, trick.id, 0, card.clone(), 0).await?;
            plays::create_play(txn, trick.id, 1, card.clone(), 1).await?;
            plays::create_play(txn, trick.id, 2, card.clone(), 2).await?;

            let count = plays::count_plays_by_trick(txn, trick.id).await?;
            assert_eq!(count, 3);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: complete trick with 4 plays
#[tokio::test]
async fn test_complete_trick_with_four_plays() -> Result<(), AppError> {
    let state = build_test_state().await.expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 5, 0).await?;
            let trick = tricks::create_trick(txn, round.id, 0, tricks::Suit::Spades, 1).await?;

            // Create 4 plays
            plays::create_play(
                txn,
                trick.id,
                0,
                plays::Card {
                    suit: "SPADES".into(),
                    rank: "SEVEN".into(),
                },
                0,
            )
            .await?;
            plays::create_play(
                txn,
                trick.id,
                1,
                plays::Card {
                    suit: "SPADES".into(),
                    rank: "ACE".into(),
                },
                1,
            )
            .await?;
            plays::create_play(
                txn,
                trick.id,
                2,
                plays::Card {
                    suit: "SPADES".into(),
                    rank: "KING".into(),
                },
                2,
            )
            .await?;
            plays::create_play(
                txn,
                trick.id,
                3,
                plays::Card {
                    suit: "CLUBS".into(),
                    rank: "TWO".into(),
                },
                3,
            )
            .await?;

            let count = plays::count_plays_by_trick(txn, trick.id).await?;
            assert_eq!(count, 4);

            let all_plays = plays::find_all_by_trick(txn, trick.id).await?;
            assert_eq!(all_plays.len(), 4);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: unique constraint on (trick_id, player_seat)
#[tokio::test]
async fn test_unique_constraint_trick_seat() -> Result<(), AppError> {
    let state = build_test_state().await.expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 5, 0).await?;
            let trick = tricks::create_trick(txn, round.id, 0, tricks::Suit::Hearts, 0).await?;

            // Create first play for seat 0
            plays::create_play(
                txn,
                trick.id,
                0,
                plays::Card {
                    suit: "HEARTS".into(),
                    rank: "ACE".into(),
                },
                0,
            )
            .await?;

            // Try to create duplicate play for same seat
            let result = plays::create_play(
                txn,
                trick.id,
                0,
                plays::Card {
                    suit: "HEARTS".into(),
                    rank: "KING".into(),
                },
                1,
            )
            .await;

            assert!(result.is_err(), "Duplicate seat should fail");

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

/// Test: unique constraint on (trick_id, play_order)
#[tokio::test]
async fn test_unique_constraint_trick_order() -> Result<(), AppError> {
    let state = build_test_state().await.expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 5, 0).await?;
            let trick = tricks::create_trick(txn, round.id, 0, tricks::Suit::Hearts, 0).await?;

            // Create first play with order 0
            plays::create_play(
                txn,
                trick.id,
                0,
                plays::Card {
                    suit: "HEARTS".into(),
                    rank: "ACE".into(),
                },
                0,
            )
            .await?;

            // Try to create another play with same play_order
            let result = plays::create_play(
                txn,
                trick.id,
                1,
                plays::Card {
                    suit: "HEARTS".into(),
                    rank: "KING".into(),
                },
                0,
            )
            .await;

            assert!(result.is_err(), "Duplicate play_order should fail");

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

/// Test: plays are ordered correctly
#[tokio::test]
async fn test_plays_ordering() -> Result<(), AppError> {
    let state = build_test_state().await.expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 4, 0).await?;
            let trick = tricks::create_trick(txn, round.id, 0, tricks::Suit::Hearts, 0).await?;

            // Create plays out of sequential order
            plays::create_play(
                txn,
                trick.id,
                2,
                plays::Card {
                    suit: "HEARTS".into(),
                    rank: "QUEEN".into(),
                },
                2,
            )
            .await?;
            plays::create_play(
                txn,
                trick.id,
                0,
                plays::Card {
                    suit: "HEARTS".into(),
                    rank: "ACE".into(),
                },
                0,
            )
            .await?;
            plays::create_play(
                txn,
                trick.id,
                3,
                plays::Card {
                    suit: "HEARTS".into(),
                    rank: "JACK".into(),
                },
                3,
            )
            .await?;
            plays::create_play(
                txn,
                trick.id,
                1,
                plays::Card {
                    suit: "HEARTS".into(),
                    rank: "KING".into(),
                },
                1,
            )
            .await?;

            // Should be returned in play_order
            let all_plays = plays::find_all_by_trick(txn, trick.id).await?;
            assert_eq!(all_plays.len(), 4);
            assert_eq!(all_plays[0].play_order, 0);
            assert_eq!(all_plays[1].play_order, 1);
            assert_eq!(all_plays[2].play_order, 2);
            assert_eq!(all_plays[3].play_order, 3);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
