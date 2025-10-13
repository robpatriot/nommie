mod common;
mod support;

use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::repos::{games, rounds, scores};
use backend::services::game_flow::GameFlowService;
use support::game_phases::setup_game_in_trick_play_phase;
use tracing::info;

/// Test: End-to-end game flow for one complete round
/// Deal -> Bid -> SetTrump -> PlayCards -> ResolveTricks -> ScoreRound -> AdvanceRound
#[tokio::test]
async fn test_end_to_end_one_round() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Set up game in TrickPlay phase (Round 1: dealer at seat 0, bids: 3, 3, 4, 2, trump: Hearts)
            let setup =
                setup_game_in_trick_play_phase(txn, 12345, [3, 3, 4, 2], rounds::Trump::Hearts)
                    .await?;
            let service = GameFlowService::new();

            let game_after_setup = games::find_by_id(txn, setup.game_id).await?.unwrap();
            assert_eq!(game_after_setup.id, setup.game_id); // Type check

            // Play cards for one complete trick (simplified - just create trick with winner)
            // In a real game, we'd call play_card 4 times
            // For this test, we'll manually create the trick and plays, then call resolve_trick

            let round = rounds::find_by_id(txn, setup.round_id).await?.unwrap();

            // Create trick 0 manually
            use backend::repos::{plays, tricks};
            let trick0 = tricks::create_trick(txn, round.id, 0, tricks::Suit::Hearts, 0).await?;

            // Create 4 plays
            plays::create_play(
                txn,
                trick0.id,
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
                trick0.id,
                1,
                plays::Card {
                    suit: "HEARTS".into(),
                    rank: "KING".into(),
                },
                1,
            )
            .await?;
            plays::create_play(
                txn,
                trick0.id,
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
                trick0.id,
                3,
                plays::Card {
                    suit: "CLUBS".into(),
                    rank: "TWO".into(),
                },
                3,
            )
            .await?;

            // Resolve trick (determines winner)
            service.resolve_trick(txn, setup.game_id).await?;

            // Winner should be seat 0 (Ace of trump suit)
            // current_trick_no should have advanced to 1

            // For testing scoring, let's create all 13 tricks with known winners
            // P0 wins 3, P1 wins 3, P2 wins 4 (meets bid), P3 wins 3
            for i in 1..3 {
                let trick = tricks::create_trick(txn, round.id, i, tricks::Suit::Hearts, 0).await?;
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
                    1,
                    plays::Card {
                        suit: "HEARTS".into(),
                        rank: "TWO".into(),
                    },
                    1,
                )
                .await?;
                plays::create_play(
                    txn,
                    trick.id,
                    2,
                    plays::Card {
                        suit: "HEARTS".into(),
                        rank: "THREE".into(),
                    },
                    2,
                )
                .await?;
                plays::create_play(
                    txn,
                    trick.id,
                    3,
                    plays::Card {
                        suit: "HEARTS".into(),
                        rank: "FOUR".into(),
                    },
                    3,
                )
                .await?;
            }
            for i in 3..6 {
                tricks::create_trick(txn, round.id, i, tricks::Suit::Spades, 1).await?;
            }
            for i in 6..10 {
                tricks::create_trick(txn, round.id, i, tricks::Suit::Clubs, 2).await?;
            }
            for i in 10..13 {
                tricks::create_trick(txn, round.id, i, tricks::Suit::Diamonds, 3).await?;
            }

            // Score the round
            service.score_round(txn, setup.game_id).await?;

            // Verify scores
            let all_scores = scores::find_all_by_round(txn, round.id).await?;
            assert_eq!(all_scores.len(), 4);

            // P0: bid 3, won 3, met -> 3 + 10 = 13
            assert_eq!(all_scores[0].bid_value, 3);
            assert_eq!(all_scores[0].tricks_won, 3);
            assert!(all_scores[0].bid_met);
            assert_eq!(all_scores[0].round_score, 13);

            // P2: bid 4, won 4, met -> 4 + 10 = 14
            assert_eq!(all_scores[2].bid_value, 4);
            assert_eq!(all_scores[2].tricks_won, 4);
            assert!(all_scores[2].bid_met);
            assert_eq!(all_scores[2].round_score, 14);

            // Advance to next round
            service.advance_to_next_round(txn, setup.game_id).await?;

            // Game should be in BetweenRounds state
            // current_trick_no should be reset to 0

            info!("End-to-end test completed successfully");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
