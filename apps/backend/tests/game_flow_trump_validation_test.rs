mod common;
mod support;

use backend::adapters::games_sea;
use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::repos::{games, rounds};
use backend::services::game_flow::GameFlowService;
use ulid::Ulid;

fn short_join_code() -> String {
    format!("{}", Ulid::new()).chars().take(10).collect()
}

/// Test: Only the bid winner can choose trump
#[tokio::test]
async fn test_only_bid_winner_can_choose_trump() -> Result<(), AppError> {
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

            // Deal round 1
            service.deal_round(txn, game.id).await?;

            // All players submit bids
            // Round 1: dealer at seat 0, bidding starts at seat 1
            // Bids: 3 + 4 + 3 + 2 = 12
            service.submit_bid(txn, game.id, 1, 3).await?;
            service.submit_bid(txn, game.id, 2, 4).await?; // Highest bid - seat 2 wins
            service.submit_bid(txn, game.id, 3, 3).await?;
            service.submit_bid(txn, game.id, 0, 2).await?; // Dealer bids last

            // After 4th bid, should be in TrumpSelection phase
            let game_after_bids = games_sea::find_by_id(txn, game.id).await?.unwrap();
            assert_eq!(
                game_after_bids.state,
                backend::entities::games::GameState::TrumpSelection
            );

            // Attempt to set trump with wrong player (seat 0, not the winner)
            let result = service
                .set_trump(txn, game.id, 0, rounds::Trump::Hearts)
                .await;

            // Should fail with OutOfTurn error
            assert!(result.is_err());
            let err = result.unwrap_err();
            let error_message = format!("{err}");
            assert!(
                error_message.contains("Only the winning bidder")
                    || error_message.contains("Out of turn"),
                "Expected OutOfTurn/bid winner error, got: {error_message}"
            );

            // Attempt with another wrong player (seat 1)
            let result = service
                .set_trump(txn, game.id, 1, rounds::Trump::Spades)
                .await;
            assert!(result.is_err());

            // Now try with the correct player (seat 2)
            let result = service
                .set_trump(txn, game.id, 2, rounds::Trump::Diamonds)
                .await;
            assert!(result.is_ok(), "Winning bidder should be able to set trump");

            // Verify trump was set
            let round = rounds::find_by_game_and_round(txn, game.id, 1)
                .await?
                .unwrap();
            assert_eq!(round.trump, Some(rounds::Trump::Diamonds));

            // Verify game transitioned to TrickPlay
            let game_after_trump = games_sea::find_by_id(txn, game.id).await?.unwrap();
            assert_eq!(
                game_after_trump.state,
                backend::entities::games::GameState::TrickPlay
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Tie in bids - earliest bidder wins
#[tokio::test]
async fn test_trump_selection_with_tied_bids() -> Result<(), AppError> {
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

            // Deal round 1
            service.deal_round(txn, game.id).await?;

            // All players submit bids with a tie
            // Round 1: dealer at seat 0, bidding starts at seat 1
            // Bids: 4 + 2 + 4 + 2 = 12
            // Seats 1 and 3 both bid 4, but seat 1 bid first
            service.submit_bid(txn, game.id, 1, 4).await?; // First to bid 4
            service.submit_bid(txn, game.id, 2, 2).await?;
            service.submit_bid(txn, game.id, 3, 4).await?; // Also bids 4, but later
            service.submit_bid(txn, game.id, 0, 2).await?; // Dealer bids last

            // Seat 1 should be the winner (earliest bidder among tied highest)
            // Only seat 1 should be able to set trump
            let result = service
                .set_trump(txn, game.id, 3, rounds::Trump::Hearts)
                .await;
            assert!(
                result.is_err(),
                "Seat 3 should not be able to set trump despite tied bid"
            );

            // Seat 1 should succeed
            let result = service
                .set_trump(txn, game.id, 1, rounds::Trump::Clubs)
                .await;
            assert!(
                result.is_ok(),
                "Seat 1 should win trump selection (earliest tied bid)"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
