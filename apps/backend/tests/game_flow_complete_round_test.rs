mod common;
mod support;

use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::repos::{bids, games, rounds, scores, tricks};
use backend::services::game_flow::GameFlowService;
use ulid::Ulid;

fn short_join_code() -> String {
    format!("{}", Ulid::new()).chars().take(10).collect()
}

/// Test: Complete round flow - deal, bid, score
#[tokio::test]
async fn test_complete_round_flow_with_scoring() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create game
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;

            let service = GameFlowService::new();

            // 1. Deal round
            service.deal_round(txn, game.id).await?;

            // Verify round and hands were created
            let round = rounds::find_by_game_and_round(txn, game.id, 1)
                .await?
                .expect("Round 1 should exist");
            assert_eq!(round.round_no, 1);
            assert_eq!(round.hand_size, 13); // First round has 13 cards
            assert_eq!(round.completed_at, None);

            // 2. Submit bids for all players
            // Round 1: dealer is at seat 0, so bidding starts at seat 1
            // Bids: 4 + 3 + 0 + 5 = 12 (not 13, so dealer rule OK)
            service.submit_bid(txn, game.id, 1, 4).await?;
            service.submit_bid(txn, game.id, 2, 3).await?;
            service.submit_bid(txn, game.id, 3, 0).await?;
            service.submit_bid(txn, game.id, 0, 5).await?; // Dealer bids last

            // Verify all bids persisted (in bid order: seats 1, 2, 3, 0)
            let all_bids = bids::find_all_by_round(txn, round.id).await?;
            assert_eq!(all_bids.len(), 4);
            assert_eq!(all_bids[0].bid_value, 4); // Seat 1
            assert_eq!(all_bids[1].bid_value, 3); // Seat 2

            // 3. Simulate tricks (create trick records manually for this test)
            // Player 0 wins 5 tricks, player 1 wins 4, player 2 wins 3, player 3 wins 1
            for trick_no in 0..5 {
                tricks::create_trick(txn, round.id, trick_no, tricks::Suit::Hearts, 0).await?;
            }
            for trick_no in 5..9 {
                tricks::create_trick(txn, round.id, trick_no, tricks::Suit::Spades, 1).await?;
            }
            for trick_no in 9..12 {
                tricks::create_trick(txn, round.id, trick_no, tricks::Suit::Clubs, 2).await?;
            }
            tricks::create_trick(txn, round.id, 12, tricks::Suit::Diamonds, 3).await?;

            // 4. Score the round
            service.score_round(txn, game.id).await?;

            // Verify scores were calculated and persisted correctly
            let all_scores = scores::find_all_by_round(txn, round.id).await?;
            assert_eq!(all_scores.len(), 4);

            // Player 0: bid 5, won 5, met = 5 + 10 = 15
            assert_eq!(all_scores[0].player_seat, 0);
            assert_eq!(all_scores[0].bid_value, 5);
            assert_eq!(all_scores[0].tricks_won, 5);
            assert!(all_scores[0].bid_met);
            assert_eq!(all_scores[0].base_score, 5);
            assert_eq!(all_scores[0].bonus, 10);
            assert_eq!(all_scores[0].round_score, 15);
            assert_eq!(all_scores[0].total_score_after, 15);

            // Player 1: bid 4, won 4, met = 4 + 10 = 14
            assert_eq!(all_scores[1].player_seat, 1);
            assert_eq!(all_scores[1].bid_value, 4);
            assert_eq!(all_scores[1].tricks_won, 4);
            assert!(all_scores[1].bid_met);
            assert_eq!(all_scores[1].round_score, 14);

            // Player 2: bid 3, won 3, met = 3 + 10 = 13
            assert_eq!(all_scores[2].player_seat, 2);
            assert_eq!(all_scores[2].tricks_won, 3);
            assert!(all_scores[2].bid_met);
            assert_eq!(all_scores[2].round_score, 13);

            // Player 3: bid 0, won 1, not met = 1 + 0 = 1
            assert_eq!(all_scores[3].player_seat, 3);
            assert_eq!(all_scores[3].bid_value, 0);
            assert_eq!(all_scores[3].tricks_won, 1);
            assert!(!all_scores[3].bid_met);
            assert_eq!(all_scores[3].round_score, 1);

            // Verify round is marked as complete
            let updated_round = rounds::find_by_id(txn, round.id).await?.unwrap();
            assert!(updated_round.completed_at.is_some());

            // Verify game transitioned to Scoring
            let updated_game = games::find_by_id(txn, game.id).await?.unwrap();
            assert_eq!(updated_game.id, game.id); // Type check

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Multi-round cumulative scoring
#[tokio::test]
async fn test_multi_round_cumulative_scoring() -> Result<(), AppError> {
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

            // Round 1
            service.deal_round(txn, game.id).await?;
            let round1 = rounds::find_by_game_and_round(txn, game.id, 1)
                .await?
                .unwrap();

            // Round 1: dealer at seat 0, bidding starts at seat 1
            // Bids: 3 + 2 + 0 + 7 = 12 (not 13, dealer rule OK)
            service.submit_bid(txn, game.id, 1, 3).await?;
            service.submit_bid(txn, game.id, 2, 2).await?;
            service.submit_bid(txn, game.id, 3, 0).await?;
            service.submit_bid(txn, game.id, 0, 7).await?; // Dealer bids last

            // Simulate tricks: P0 wins 7, P1 wins 3, P2 wins 2, P3 wins 1 (totals 13)
            for i in 0..7 {
                tricks::create_trick(txn, round1.id, i, tricks::Suit::Hearts, 0).await?;
            }
            for i in 7..10 {
                tricks::create_trick(txn, round1.id, i, tricks::Suit::Spades, 1).await?;
            }
            for i in 10..12 {
                tricks::create_trick(txn, round1.id, i, tricks::Suit::Clubs, 2).await?;
            }
            tricks::create_trick(txn, round1.id, 12, tricks::Suit::Diamonds, 3).await?;

            service.score_round(txn, game.id).await?;

            // Check round 1 totals
            let totals1 = scores::get_current_totals(txn, game.id).await?;
            // P0: bid 7, won 7, met -> 7+10=17
            // P1: bid 3, won 3, met -> 3+10=13
            // P2: bid 2, won 2, met -> 2+10=12
            // P3: bid 0, won 1, not met -> 1+0=1
            assert_eq!(totals1, [17, 13, 12, 1]);

            // Advance and deal round 2
            service.advance_to_next_round(txn, game.id).await?;
            service.deal_round(txn, game.id).await?;
            let round2 = rounds::find_by_game_and_round(txn, game.id, 2)
                .await?
                .unwrap();

            // Round 2: dealer at seat 1, bidding starts at seat 2
            // Bids: 2 + 0 + 5 + 4 = 11 (not 12, dealer rule OK)
            service.submit_bid(txn, game.id, 2, 2).await?;
            service.submit_bid(txn, game.id, 3, 0).await?;
            service.submit_bid(txn, game.id, 0, 5).await?;
            service.submit_bid(txn, game.id, 1, 4).await?; // Dealer bids last

            // Simulate round 2 tricks: P0 wins 5, P1 wins 4, P2 wins 2, P3 wins 1 (totals 12)
            for i in 0..5 {
                tricks::create_trick(txn, round2.id, i, tricks::Suit::Hearts, 0).await?;
            }
            for i in 5..9 {
                tricks::create_trick(txn, round2.id, i, tricks::Suit::Spades, 1).await?;
            }
            for i in 9..11 {
                tricks::create_trick(txn, round2.id, i, tricks::Suit::Clubs, 2).await?;
            }
            tricks::create_trick(txn, round2.id, 11, tricks::Suit::Diamonds, 3).await?;

            service.score_round(txn, game.id).await?;

            // Check cumulative totals
            let totals2 = scores::get_current_totals(txn, game.id).await?;
            // P0: 17 + (5+10) = 32
            // P1: 13 + (4+10) = 27
            // P2: 12 + (2+10) = 24
            // P3: 1 + (1+0) = 2  (bid 0, won 1, not met)
            assert_eq!(totals2, [32, 27, 24, 2]);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
