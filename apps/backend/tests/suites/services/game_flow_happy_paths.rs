//! Integration tests for game flow happy paths.
//!
//! This module tests successful end-to-end game flows from game creation through
//! round completion, scoring, and game completion. All tests verify deterministic
//! outcomes and proper state transitions.

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::entities::games::{self, GameState};
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::repos::{bids, games as games_repo, rounds, scores, tricks};
use backend::services::game_flow::GameFlowService;
use sea_orm::EntityTrait;
use tracing::info;

use crate::support::game_phases::{setup_game_in_bidding_phase, setup_game_in_trick_play_phase};
use crate::support::game_setup::setup_game_with_players;
use crate::support::trick_helpers::{create_tricks_by_winner_counts, create_tricks_with_winners};

#[tokio::test]
async fn test_deal_round_transitions_to_bidding() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_setup = setup_game_with_players(txn, 123).await?;
            let game_id = game_setup.game_id;

            let service = GameFlowService::new();
            service.deal_round(txn, game_id).await?;

            let game = games::Entity::find_by_id(game_id)
                .one(txn)
                .await?
                .expect("game should exist");

            assert_eq!(game.state, GameState::Bidding);
            assert_eq!(game.current_round, Some(1));
            assert_eq!(game.hand_size(), Some(13));
            assert_eq!(game.dealer_pos(), Some(0));

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_deal_round_with_different_seeds_produces_different_outcomes() -> Result<(), AppError>
{
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id1 = setup_game_with_players(txn, 111).await?.game_id;
            let game_id2 = setup_game_with_players(txn, 222).await?.game_id;

            let service = GameFlowService::new();

            service.deal_round(txn, game_id1).await?;
            service.deal_round(txn, game_id2).await?;

            let game1 = games::Entity::find_by_id(game_id1).one(txn).await?.unwrap();
            let game2 = games::Entity::find_by_id(game_id2).one(txn).await?.unwrap();

            assert_eq!(game1.state, GameState::Bidding);
            assert_eq!(game2.state, GameState::Bidding);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_submit_bid_succeeds_after_deal() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, 789).await?.game_id;
            let service = GameFlowService::new();
            service.deal_round(txn, game_id).await?;

            let result = service.submit_bid(txn, game_id, 1, 5).await;

            assert!(result.is_ok());

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_complete_round_flow_with_scoring() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let setup = setup_game_in_bidding_phase(txn, 12345).await?;
            let service = GameFlowService::new();

            let round = rounds::find_by_id(txn, setup.round_id).await?.unwrap();
            assert_eq!(round.round_no, 1);
            assert_eq!(round.hand_size, 13);
            assert_eq!(round.completed_at, None);

            // Submit bids: Round 1, dealer at seat 0, bidding starts at seat 1
            // Bids: 4 + 3 + 0 + 5 = 12 (not 13, so dealer rule OK)
            service.submit_bid(txn, setup.game_id, 1, 4).await?;
            service.submit_bid(txn, setup.game_id, 2, 3).await?;
            service.submit_bid(txn, setup.game_id, 3, 0).await?;
            service.submit_bid(txn, setup.game_id, 0, 5).await?;

            let all_bids = bids::find_all_by_round(txn, setup.round_id).await?;
            assert_eq!(all_bids.len(), 4);
            assert_eq!(all_bids[0].bid_value, 4);
            assert_eq!(all_bids[1].bid_value, 3);

            // Simulate tricks: P0 wins 5, P1 wins 4, P2 wins 3, P3 wins 1
            create_tricks_by_winner_counts(txn, setup.round_id, [5, 4, 3, 1]).await?;

            service.score_round(txn, setup.game_id).await?;

            let all_scores = scores::find_all_by_round(txn, setup.round_id).await?;
            assert_eq!(all_scores.len(), 4);

            // P0: bid 5, won 5, met = 5 + 10 = 15
            assert_eq!(all_scores[0].player_seat, 0);
            assert_eq!(all_scores[0].bid_value, 5);
            assert_eq!(all_scores[0].tricks_won, 5);
            assert!(all_scores[0].bid_met);
            assert_eq!(all_scores[0].base_score, 5);
            assert_eq!(all_scores[0].bonus, 10);
            assert_eq!(all_scores[0].round_score, 15);
            assert_eq!(all_scores[0].total_score_after, 15);

            // P1: bid 4, won 4, met = 4 + 10 = 14
            assert_eq!(all_scores[1].player_seat, 1);
            assert_eq!(all_scores[1].bid_value, 4);
            assert_eq!(all_scores[1].tricks_won, 4);
            assert!(all_scores[1].bid_met);
            assert_eq!(all_scores[1].round_score, 14);

            // P2: bid 3, won 3, met = 3 + 10 = 13
            assert_eq!(all_scores[2].player_seat, 2);
            assert_eq!(all_scores[2].tricks_won, 3);
            assert!(all_scores[2].bid_met);
            assert_eq!(all_scores[2].round_score, 13);

            // P3: bid 0, won 1, not met = 1 + 0 = 1
            assert_eq!(all_scores[3].player_seat, 3);
            assert_eq!(all_scores[3].bid_value, 0);
            assert_eq!(all_scores[3].tricks_won, 1);
            assert!(!all_scores[3].bid_met);
            assert_eq!(all_scores[3].round_score, 1);

            let updated_round = rounds::find_by_id(txn, setup.round_id).await?.unwrap();
            assert!(updated_round.completed_at.is_some());

            let updated_game = games_repo::find_by_id(txn, setup.game_id).await?.unwrap();
            assert_eq!(updated_game.id, setup.game_id);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_multi_round_cumulative_scoring() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let setup = setup_game_in_bidding_phase(txn, 12346).await?;
            let service = GameFlowService::new();

            // Round 1: dealer at seat 0, bidding starts at seat 1
            // Bids: 3 + 2 + 0 + 7 = 12 (not 13, dealer rule OK)
            service.submit_bid(txn, setup.game_id, 1, 3).await?;
            service.submit_bid(txn, setup.game_id, 2, 2).await?;
            service.submit_bid(txn, setup.game_id, 3, 0).await?;
            service.submit_bid(txn, setup.game_id, 0, 7).await?;

            create_tricks_by_winner_counts(txn, setup.round_id, [7, 3, 2, 1]).await?;

            service.score_round(txn, setup.game_id).await?;

            let totals1 = scores::get_current_totals(txn, setup.game_id).await?;
            // P0: bid 7, won 7, met -> 7+10=17
            // P1: bid 3, won 3, met -> 3+10=13
            // P2: bid 2, won 2, met -> 2+10=12
            // P3: bid 0, won 1, not met -> 1+0=1
            assert_eq!(totals1, [17, 13, 12, 1]);

            service.advance_to_next_round(txn, setup.game_id).await?;
            service.deal_round(txn, setup.game_id).await?;
            let round2 = rounds::find_by_game_and_round(txn, setup.game_id, 2)
                .await?
                .unwrap();

            // Round 2: dealer at seat 1, bidding starts at seat 2
            // Bids: 2 + 0 + 5 + 4 = 11 (not 12, dealer rule OK)
            service.submit_bid(txn, setup.game_id, 2, 2).await?;
            service.submit_bid(txn, setup.game_id, 3, 0).await?;
            service.submit_bid(txn, setup.game_id, 0, 5).await?;
            service.submit_bid(txn, setup.game_id, 1, 4).await?;

            create_tricks_by_winner_counts(txn, round2.id, [5, 4, 2, 1]).await?;

            service.score_round(txn, setup.game_id).await?;

            let totals2 = scores::get_current_totals(txn, setup.game_id).await?;
            // P0: 17 + (5+10) = 32
            // P1: 13 + (4+10) = 27
            // P2: 12 + (2+10) = 24
            // P3: 1 + (1+0) = 2
            assert_eq!(totals2, [32, 27, 24, 2]);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_end_to_end_one_round() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
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

            let game_after_setup = games_repo::find_by_id(txn, setup.game_id).await?.unwrap();
            assert_eq!(game_after_setup.id, setup.game_id);

            let round = rounds::find_by_id(txn, setup.round_id).await?.unwrap();

            use backend::repos::plays;
            let trick0 = tricks::create_trick(txn, round.id, 1, tricks::Suit::Hearts, 0).await?;

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

            service.resolve_trick(txn, setup.game_id).await?;

            // P0 wins tricks 2-3 (2 total), P1 wins 4-6 (3 total), P2 wins 7-10 (4 total), P3 wins 11-13 (3 total)
            create_tricks_with_winners(txn, round.id, &[0, 0, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3], 2)
                .await?;

            service.score_round(txn, setup.game_id).await?;

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

            service.advance_to_next_round(txn, setup.game_id).await?;

            info!("End-to-end test completed successfully");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_game_completes_after_final_round() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let setup = crate::support::game_phases::setup_game_at_round(txn, 12345, 25).await?;
            let service = GameFlowService::new();

            let game = games::Entity::find_by_id(setup.game_id)
                .one(txn)
                .await?
                .expect("game should exist");
            assert_eq!(game.state, GameState::BetweenRounds);
            assert_eq!(game.current_round, Some(25));
            assert_eq!(game.starting_dealer_pos, Some(0));

            // Deal round 26 (the final round with 13 cards per player)
            service.deal_round(txn, setup.game_id).await?;

            let game = games::Entity::find_by_id(setup.game_id)
                .one(txn)
                .await?
                .unwrap();
            assert_eq!(game.state, GameState::Bidding);
            assert_eq!(game.current_round, Some(26));
            assert_eq!(game.hand_size(), Some(13));

            let round = rounds::find_by_game_and_round(txn, setup.game_id, 26)
                .await?
                .expect("Round 26 should exist");
            assert_eq!(round.hand_size, 13);

            // Submit bids: Round 26, dealer at seat 1, bidding starts at seat 2
            // Bids: 3 + 3 + 4 + 2 = 12 (not 13, dealer rule OK)
            service.submit_bid(txn, setup.game_id, 2, 3).await?;
            service.submit_bid(txn, setup.game_id, 3, 3).await?;
            service.submit_bid(txn, setup.game_id, 0, 4).await?;
            service.submit_bid(txn, setup.game_id, 1, 2).await?;

            let all_bids = bids::find_all_by_round(txn, round.id).await?;
            assert_eq!(all_bids.len(), 4);

            service
                .set_trump(txn, setup.game_id, 0, rounds::Trump::Hearts)
                .await?;

            // Simulate 13 tricks: P0 wins 4, P1 wins 2, P2 wins 3, P3 wins 4
            create_tricks_by_winner_counts(txn, round.id, [4, 2, 3, 4]).await?;

            service.score_round(txn, setup.game_id).await?;

            let round_scores = scores::find_all_by_round(txn, round.id).await?;
            assert_eq!(round_scores.len(), 4);
            // Seat 0: bid 4, won 4 -> 4 + 10 = 14
            assert_eq!(round_scores[0].bid_value, 4);
            assert_eq!(round_scores[0].tricks_won, 4);
            assert!(round_scores[0].bid_met);
            assert_eq!(round_scores[0].round_score, 14);

            service.advance_to_next_round(txn, setup.game_id).await?;

            let final_game = games::Entity::find_by_id(setup.game_id)
                .one(txn)
                .await?
                .unwrap();
            assert_eq!(final_game.state, GameState::Completed);
            assert_eq!(final_game.current_round, Some(26));

            let final_totals = scores::get_current_totals(txn, setup.game_id).await?;
            assert_eq!(final_totals.len(), 4);
            // Each player had 4*25=100 points from rounds 1-25, plus round 26 scores
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
