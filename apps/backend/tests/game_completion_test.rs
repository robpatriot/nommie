mod common;
mod support;

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::entities::games::{self, GameState};
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::repos::{bids, rounds, scores, tricks};
use backend::services::game_flow::GameFlowService;
use sea_orm::EntityTrait;
use support::game_phases;

/// Test: Game completes successfully after final round (round 26)
///
/// This test verifies that:
/// 1. A game at round 25 can advance to round 26 (final round with 13 cards)
/// 2. Round 26 can be played through completely (bid, trump, play, score)
/// 3. Game transitions to Completed state after round 26 is scored
/// 4. Final scores are calculated correctly
#[tokio::test]
async fn test_game_completes_after_final_round() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create game with 25 rounds already completed using helper
            let setup = game_phases::setup_game_at_round(txn, 12345, 25).await?;
            let service = GameFlowService::new();

            // Verify game is in BetweenRounds state, ready for round 26
            let game = games::Entity::find_by_id(setup.game_id)
                .one(txn)
                .await?
                .expect("game should exist");
            assert_eq!(
                game.state,
                GameState::BetweenRounds,
                "Game should be in BetweenRounds state"
            );
            assert_eq!(
                game.current_round,
                Some(25),
                "Game should be at round 25 before dealing"
            );
            assert_eq!(
                game.starting_dealer_pos,
                Some(0),
                "Starting dealer should be set"
            );

            // Deal round 26 (the final round with 13 cards per player)
            service.deal_round(txn, setup.game_id).await?;

            let game = games::Entity::find_by_id(setup.game_id)
                .one(txn)
                .await?
                .unwrap();
            assert_eq!(
                game.state,
                GameState::Bidding,
                "Game should be in Bidding state after dealing"
            );
            assert_eq!(
                game.current_round,
                Some(26),
                "Game should be at round 26 after dealing round 26"
            );
            assert_eq!(
                game.hand_size(),
                Some(13),
                "Final round (26) should have 13 cards"
            );

            // Get the round
            let round = rounds::find_by_game_and_round(txn, setup.game_id, 26)
                .await?
                .expect("Round 26 should exist");
            assert_eq!(round.hand_size, 13);

            // Submit bids for all players (dealer at seat 1 for round 26)
            // Round 26: dealer at seat (25 % 4) = seat 1
            // Bidding starts at seat 2
            // Bids must sum to not equal hand_size (13), so use total != 13
            service.submit_bid(txn, setup.game_id, 2, 3).await?;
            service.submit_bid(txn, setup.game_id, 3, 3).await?;
            service.submit_bid(txn, setup.game_id, 0, 4).await?;
            service.submit_bid(txn, setup.game_id, 1, 2).await?; // Dealer bids last (3+3+4+2=12, not 13)

            // Verify bids were recorded
            let all_bids = bids::find_all_by_round(txn, round.id).await?;
            assert_eq!(all_bids.len(), 4);

            // Set trump (winning bidder is seat 0 with bid of 4)
            service
                .set_trump(txn, setup.game_id, 0, rounds::Trump::Hearts)
                .await?;

            // Simulate 13 tricks with known winners to match bids
            // P0 bid 4, P1 bid 2, P2 bid 3, P3 bid 3 -> need 13 total tricks won, only P0 meets bid
            for i in 0..4 {
                tricks::create_trick(txn, round.id, i, tricks::Suit::Hearts, 0).await?;
                // P0 wins 4
            }
            for i in 4..6 {
                tricks::create_trick(txn, round.id, i, tricks::Suit::Spades, 1).await?;
                // P1 wins 2
            }
            for i in 6..9 {
                tricks::create_trick(txn, round.id, i, tricks::Suit::Clubs, 2).await?;
                // P2 wins 3
            }
            for i in 9..13 {
                tricks::create_trick(txn, round.id, i, tricks::Suit::Diamonds, 3).await?;
                // P3 wins 4 (doesn't meet bid of 3)
            }

            // Score the round
            service.score_round(txn, setup.game_id).await?;

            // Verify round 26 scores
            let round_scores = scores::find_all_by_round(txn, round.id).await?;
            assert_eq!(round_scores.len(), 4);
            // Seat 0: bid 4, won 4 -> 4 + 10 = 14
            assert_eq!(round_scores[0].bid_value, 4);
            assert_eq!(round_scores[0].tricks_won, 4);
            assert!(round_scores[0].bid_met);
            assert_eq!(round_scores[0].round_score, 14);

            // Advance to next round - this should trigger game completion
            service.advance_to_next_round(txn, setup.game_id).await?;

            // Verify game is now in Completed state
            let final_game = games::Entity::find_by_id(setup.game_id)
                .one(txn)
                .await?
                .unwrap();
            assert_eq!(
                final_game.state,
                GameState::Completed,
                "Game should transition to Completed after round 26"
            );
            assert_eq!(final_game.current_round, Some(26));

            // Verify final cumulative scores are reasonable
            let final_totals = scores::get_current_totals(txn, setup.game_id).await?;
            assert_eq!(final_totals.len(), 4);
            // Each player had 4*25=100 points from rounds 1-25, plus round 26 scores
            // Round 26: P0 bid 4 won 4 (met: 14), P1 bid 2 won 2 (met: 12), P2 bid 3 won 3 (met: 13), P3 bid 3 won 4 (not met: 4)
            assert_eq!(final_totals[0], 114); // 100 + 14
            assert_eq!(final_totals[1], 112); // 100 + 12
            assert_eq!(final_totals[2], 113); // 100 + 13
            assert_eq!(final_totals[3], 104); // 100 + 4

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
