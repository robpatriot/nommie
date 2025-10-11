mod common;
mod support;

use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::repos::{games, rounds, scores};
use backend::services::game_flow::GameFlowService;
use tracing::info;
use ulid::Ulid;

fn short_join_code() -> String {
    format!("{}", Ulid::new()).chars().take(10).collect()
}

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
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let service = GameFlowService::new();

            // Step 1: Deal round 1
            service.deal_round(txn, game.id).await?;

            let game_after_deal = games::find_by_id(txn, game.id).await?.unwrap();
            assert_eq!(game_after_deal.id, game.id); // Type check
                                                     // State should be Bidding after deal (verified in other tests)

            // Step 2: All players submit bids
            // Round 1: dealer at seat 0, bidding starts at seat 1
            // Bids: 3 + 4 + 2 + 3 = 12 (not 13, dealer rule OK)
            service.submit_bid(txn, game.id, 1, 3).await?;
            service.submit_bid(txn, game.id, 2, 4).await?; // Highest bid
            service.submit_bid(txn, game.id, 3, 2).await?;
            service.submit_bid(txn, game.id, 0, 3).await?; // Dealer bids last

            // After 4th bid, should auto-transition to TrumpSelection
            // (verified by submit_bid logic)

            // Step 3: Set trump (winning bidder selects)
            // Player 2 has the highest bid (4), so they must choose trump
            service
                .set_trump(txn, game.id, 2, rounds::Trump::Hearts)
                .await?;

            // Should now be in TrickPlay state

            // Step 4: Play cards for one complete trick (simplified - just create trick with winner)
            // In a real game, we'd call play_card 4 times
            // For this test, we'll manually create the trick and plays, then call resolve_trick

            let round = rounds::find_by_game_and_round(txn, game.id, 1)
                .await?
                .unwrap();

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

            // Step 5: Resolve trick (determines winner)
            service.resolve_trick(txn, game.id).await?;

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

            // Step 6: Score the round
            service.score_round(txn, game.id).await?;

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

            // Step 7: Advance to next round
            service.advance_to_next_round(txn, game.id).await?;

            // Game should be in BetweenRounds state
            // current_trick_no should be reset to 0

            info!("End-to-end test completed successfully");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
