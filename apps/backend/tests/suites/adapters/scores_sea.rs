use backend::adapters::games_sea::GameCreate;
use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::repos::{games, rounds, scores};
use backend::utils::join_code::generate_join_code;

use crate::support::build_test_state;

/// Test: create_score and find_by_round_and_seat
#[tokio::test]
async fn test_create_score_and_find() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            // Create a score
            let score = scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round.id,
                    player_seat: 0,
                    bid_value: 5,
                    tricks_won: 5,
                    bid_met: true,
                    base_score: 5,
                    bonus: 10,
                    round_score: 15,
                    total_score_after: 15,
                },
            )
            .await?;

            assert!(score.id > 0);
            assert_eq!(score.player_seat, 0);
            assert_eq!(score.bid_value, 5);
            assert_eq!(score.tricks_won, 5);
            assert!(score.bid_met);
            assert_eq!(score.base_score, 5);
            assert_eq!(score.bonus, 10);
            assert_eq!(score.round_score, 15);
            assert_eq!(score.total_score_after, 15);

            // Find by round and seat
            let found = scores::find_by_round_and_seat(txn, round.id, 0).await?;
            assert!(found.is_some());
            let found = found.unwrap();
            assert_eq!(found.id, score.id);
            assert_eq!(found.round_score, 15);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_all_by_round returns scores ordered by seat
#[tokio::test]
async fn test_find_all_by_round_ordered() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;
            let round = rounds::create_round(txn, game.id, 1, 5, 0).await?;

            // Create scores out of order
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round.id,
                    player_seat: 2,
                    bid_value: 3,
                    tricks_won: 3,
                    bid_met: true,
                    base_score: 3,
                    bonus: 10,
                    round_score: 13,
                    total_score_after: 13,
                },
            )
            .await?;
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round.id,
                    player_seat: 0,
                    bid_value: 5,
                    tricks_won: 4,
                    bid_met: false,
                    base_score: 4,
                    bonus: 0,
                    round_score: 4,
                    total_score_after: 4,
                },
            )
            .await?;
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round.id,
                    player_seat: 3,
                    bid_value: 2,
                    tricks_won: 2,
                    bid_met: true,
                    base_score: 2,
                    bonus: 10,
                    round_score: 12,
                    total_score_after: 12,
                },
            )
            .await?;
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round.id,
                    player_seat: 1,
                    bid_value: 4,
                    tricks_won: 5,
                    bid_met: false,
                    base_score: 5,
                    bonus: 0,
                    round_score: 5,
                    total_score_after: 5,
                },
            )
            .await?;

            // Find all should return ordered by seat
            let all_scores = scores::find_all_by_round(txn, round.id).await?;
            assert_eq!(all_scores.len(), 4);
            assert_eq!(all_scores[0].player_seat, 0);
            assert_eq!(all_scores[1].player_seat, 1);
            assert_eq!(all_scores[2].player_seat, 2);
            assert_eq!(all_scores[3].player_seat, 3);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: get_current_totals retrieves latest round totals
#[tokio::test]
async fn test_get_current_totals() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;

            // Round 1
            let round1 = rounds::create_round(txn, game.id, 1, 13, 0).await?;
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round1.id,
                    player_seat: 0,
                    bid_value: 5,
                    tricks_won: 5,
                    bid_met: true,
                    base_score: 5,
                    bonus: 10,
                    round_score: 15,
                    total_score_after: 15,
                },
            )
            .await?;
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round1.id,
                    player_seat: 1,
                    bid_value: 4,
                    tricks_won: 3,
                    bid_met: false,
                    base_score: 3,
                    bonus: 0,
                    round_score: 3,
                    total_score_after: 3,
                },
            )
            .await?;
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round1.id,
                    player_seat: 2,
                    bid_value: 3,
                    tricks_won: 3,
                    bid_met: true,
                    base_score: 3,
                    bonus: 10,
                    round_score: 13,
                    total_score_after: 13,
                },
            )
            .await?;
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round1.id,
                    player_seat: 3,
                    bid_value: 1,
                    tricks_won: 2,
                    bid_met: false,
                    base_score: 2,
                    bonus: 0,
                    round_score: 2,
                    total_score_after: 2,
                },
            )
            .await?;
            // Mark round 1 as complete (scores can only exist for completed rounds)
            rounds::complete_round(txn, round1.id).await?;

            // Round 2 (cumulative totals)
            let round2 = rounds::create_round(txn, game.id, 2, 12, 1).await?;
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round2.id,
                    player_seat: 0,
                    bid_value: 6,
                    tricks_won: 6,
                    bid_met: true,
                    base_score: 6,
                    bonus: 10,
                    round_score: 16,
                    total_score_after: 31,
                },
            )
            .await?;
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round2.id,
                    player_seat: 1,
                    bid_value: 5,
                    tricks_won: 4,
                    bid_met: false,
                    base_score: 4,
                    bonus: 0,
                    round_score: 4,
                    total_score_after: 7,
                },
            )
            .await?;
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round2.id,
                    player_seat: 2,
                    bid_value: 4,
                    tricks_won: 4,
                    bid_met: true,
                    base_score: 4,
                    bonus: 10,
                    round_score: 14,
                    total_score_after: 27,
                },
            )
            .await?;
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round2.id,
                    player_seat: 3,
                    bid_value: 2,
                    tricks_won: 2,
                    bid_met: true,
                    base_score: 2,
                    bonus: 10,
                    round_score: 12,
                    total_score_after: 14,
                },
            )
            .await?;
            // Mark round 2 as complete (scores can only exist for completed rounds)
            rounds::complete_round(txn, round2.id).await?;

            // Get current totals should return round 2 totals
            let totals = scores::get_current_totals(txn, game.id).await?;
            assert_eq!(totals, [31, 7, 27, 14]);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: get_current_totals returns zeros for new game
#[tokio::test]
async fn test_get_current_totals_no_rounds() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;

            // No rounds created yet
            let totals = scores::get_current_totals(txn, game.id).await?;
            assert_eq!(totals, [0, 0, 0, 0]);

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
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;
            let round = rounds::create_round(txn, game.id, 1, 5, 0).await?;

            // Create first score for seat 0
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round.id,
                    player_seat: 0,
                    bid_value: 3,
                    tricks_won: 3,
                    bid_met: true,
                    base_score: 3,
                    bonus: 10,
                    round_score: 13,
                    total_score_after: 13,
                },
            )
            .await?;

            // Try to create duplicate score for same seat
            let result = scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round.id,
                    player_seat: 0,
                    bid_value: 4,
                    tricks_won: 4,
                    bid_met: true,
                    base_score: 4,
                    bonus: 10,
                    round_score: 14,
                    total_score_after: 14,
                },
            )
            .await;

            assert!(result.is_err(), "Duplicate score should fail");

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

/// Test: bid_met flag accuracy
#[tokio::test]
async fn test_bid_met_flag() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;
            let round = rounds::create_round(txn, game.id, 1, 5, 0).await?;

            // Met bid
            let met = scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round.id,
                    player_seat: 0,
                    bid_value: 3,
                    tricks_won: 3,
                    bid_met: true,
                    base_score: 3,
                    bonus: 10,
                    round_score: 13,
                    total_score_after: 13,
                },
            )
            .await?;
            assert!(met.bid_met);
            assert_eq!(met.bonus, 10);

            // Missed bid
            let missed = scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round.id,
                    player_seat: 1,
                    bid_value: 4,
                    tricks_won: 2,
                    bid_met: false,
                    base_score: 2,
                    bonus: 0,
                    round_score: 2,
                    total_score_after: 2,
                },
            )
            .await?;
            assert!(!missed.bid_met);
            assert_eq!(missed.bonus, 0);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
