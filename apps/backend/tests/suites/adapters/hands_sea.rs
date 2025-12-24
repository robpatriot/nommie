use backend::adapters::games_sea::GameCreate;
use backend::db::txn::with_txn;
use backend::repos::{games, hands, rounds};
use backend::AppError;

use crate::support::build_test_state;

/// Test: create_hands and find_by_round_and_seat roundtrip
#[tokio::test]
async fn test_create_hands_and_find_by_seat() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game = games::create_game(txn, GameCreate::new()).await?;
            let round = rounds::create_round(txn, game.id, 1, 3, 0).await?;

            // Create hands for players
            let cards1 = vec![
                hands::Card {
                    suit: "HEARTS".into(),
                    rank: "ACE".into(),
                },
                hands::Card {
                    suit: "SPADES".into(),
                    rank: "KING".into(),
                },
                hands::Card {
                    suit: "CLUBS".into(),
                    rank: "QUEEN".into(),
                },
            ];
            let cards2 = vec![
                hands::Card {
                    suit: "DIAMONDS".into(),
                    rank: "JACK".into(),
                },
                hands::Card {
                    suit: "HEARTS".into(),
                    rank: "TEN".into(),
                },
                hands::Card {
                    suit: "SPADES".into(),
                    rank: "NINE".into(),
                },
            ];

            let created = hands::create_hands(
                txn,
                round.id,
                vec![(0, cards1.clone()), (1, cards2.clone())],
            )
            .await?;

            assert_eq!(created.len(), 2);
            assert_eq!(created[0].player_seat, 0);
            assert_eq!(created[0].cards, cards1);
            assert_eq!(created[1].player_seat, 1);
            assert_eq!(created[1].cards, cards2);

            // Find by seat
            let found = hands::find_by_round_and_seat(txn, round.id, 0).await?;
            assert!(found.is_some());
            let found = found.unwrap();
            assert_eq!(found.cards.len(), 3);
            assert_eq!(found.cards, cards1);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_all_by_round returns all hands
#[tokio::test]
async fn test_find_all_by_round() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game = games::create_game(txn, GameCreate::new()).await?;
            let round = rounds::create_round(txn, game.id, 1, 2, 0).await?;

            // Create hands for 4 players
            let hands_data = vec![
                (
                    0,
                    vec![hands::Card {
                        suit: "HEARTS".into(),
                        rank: "ACE".into(),
                    }],
                ),
                (
                    1,
                    vec![hands::Card {
                        suit: "SPADES".into(),
                        rank: "KING".into(),
                    }],
                ),
                (
                    2,
                    vec![hands::Card {
                        suit: "CLUBS".into(),
                        rank: "QUEEN".into(),
                    }],
                ),
                (
                    3,
                    vec![hands::Card {
                        suit: "DIAMONDS".into(),
                        rank: "JACK".into(),
                    }],
                ),
            ];

            hands::create_hands(txn, round.id, hands_data).await?;

            // Find all
            let all_hands = hands::find_all_by_round(txn, round.id).await?;
            assert_eq!(all_hands.len(), 4);

            // Verify all seats are present
            let seats: Vec<u8> = all_hands.iter().map(|h| h.player_seat).collect();
            assert!(seats.contains(&0));
            assert!(seats.contains(&1));
            assert!(seats.contains(&2));
            assert!(seats.contains(&3));

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_by_round_and_seat returns None for non-existent hand
#[tokio::test]
async fn test_find_by_round_and_seat_not_found() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game = games::create_game(txn, GameCreate::new()).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            // No hands created, should return None
            let found = hands::find_by_round_and_seat(txn, round.id, 0).await?;
            assert!(found.is_none());

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: empty hands (player has no cards)
#[tokio::test]
async fn test_empty_hand() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game = games::create_game(txn, GameCreate::new()).await?;
            let round = rounds::create_round(txn, game.id, 1, 0, 0).await?;

            // Create hand with no cards
            hands::create_hands(txn, round.id, vec![(0, vec![])]).await?;

            let found = hands::find_by_round_and_seat(txn, round.id, 0).await?;
            assert!(found.is_some());
            let found = found.unwrap();
            assert_eq!(found.cards.len(), 0);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: unique constraint on (round_id, player_seat)
#[tokio::test]
async fn test_unique_constraint_round_seat() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game = games::create_game(txn, GameCreate::new()).await?;
            let round = rounds::create_round(txn, game.id, 1, 3, 0).await?;

            // Create first hand for seat 0
            hands::create_hands(
                txn,
                round.id,
                vec![(
                    0,
                    vec![hands::Card {
                        suit: "HEARTS".into(),
                        rank: "ACE".into(),
                    }],
                )],
            )
            .await?;

            // Try to create duplicate hand for same seat
            let result = hands::create_hands(
                txn,
                round.id,
                vec![(
                    0,
                    vec![hands::Card {
                        suit: "SPADES".into(),
                        rank: "KING".into(),
                    }],
                )],
            )
            .await;

            assert!(result.is_err(), "Duplicate hand should fail");

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
