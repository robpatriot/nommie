mod common;
mod support;

use backend::adapters::games_sea;
use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::repos::rounds;
use backend::services::game_flow::GameFlowService;
use support::game_phases::setup_game_in_trump_selection_phase;

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
            // Set up game in TrumpSelection phase (dealt + bids submitted)
            // Round 1: dealer at seat 0, bids: 3 + 4 + 3 + 2 = 12
            let setup = setup_game_in_trump_selection_phase(txn, 12345, [2, 3, 4, 3]).await?;
            let service = GameFlowService::new();

            // After 4th bid, should be in TrumpSelection phase
            let game_after_bids = games_sea::find_by_id(txn, setup.game_id).await?.unwrap();
            assert_eq!(
                game_after_bids.state,
                backend::entities::games::GameState::TrumpSelection
            );

            // Attempt to set trump with wrong player (seat 0, not the winner)
            let result = service
                .set_trump(txn, setup.game_id, 0, rounds::Trump::Hearts)
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
                .set_trump(txn, setup.game_id, 1, rounds::Trump::Spades)
                .await;
            assert!(result.is_err());

            // Now try with the correct player (seat 2)
            let result = service
                .set_trump(txn, setup.game_id, 2, rounds::Trump::Diamonds)
                .await;
            assert!(result.is_ok(), "Winning bidder should be able to set trump");

            // Verify trump was set
            let round = rounds::find_by_game_and_round(txn, setup.game_id, 1)
                .await?
                .unwrap();
            assert_eq!(round.trump, Some(rounds::Trump::Diamonds));

            // Verify game transitioned to TrickPlay
            let game_after_trump = games_sea::find_by_id(txn, setup.game_id).await?.unwrap();
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
            // Set up game in TrumpSelection phase with tied bids
            // Round 1: dealer at seat 0, bidding starts at seat 1
            // Bids: 4 + 2 + 4 + 2 = 12
            // Seats 1 and 3 both bid 4, but seat 1 bid first
            let setup = setup_game_in_trump_selection_phase(txn, 12346, [2, 4, 2, 4]).await?;
            let service = GameFlowService::new();

            // Seat 1 should be the winner (earliest bidder among tied highest)
            // Only seat 1 should be able to set trump
            let result = service
                .set_trump(txn, setup.game_id, 3, rounds::Trump::Hearts)
                .await;
            assert!(
                result.is_err(),
                "Seat 3 should not be able to set trump despite tied bid"
            );

            // Seat 1 should succeed
            let result = service
                .set_trump(txn, setup.game_id, 1, rounds::Trump::Clubs)
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
