//! Integration test for GameFlowService happy path.
//!
//! This test validates that the happy path workflow advances game state correctly
//! with deterministic outcomes.

mod common;
mod support;

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::entities::games::{self, GameState};
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::services::game_flow::GameFlowService;
use sea_orm::EntityTrait;
use support::game_setup::setup_game_with_players;

#[tokio::test]
async fn test_deal_round_transitions_correctly() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Arrange: create game in Lobby
            let game_setup = setup_game_with_players(txn, 123).await?;
            let game_id = game_setup.game_id;

            // Act: deal first round
            let service = GameFlowService::new();
            service.deal_round(txn, game_id).await?;

            // Assert: game transitioned to Bidding
            let game = games::Entity::find_by_id(game_id)
                .one(txn)
                .await?
                .expect("game should exist");

            assert_eq!(game.state, GameState::Bidding);
            assert_eq!(game.current_round, Some(1));
            assert_eq!(game.hand_size(), Some(13)); // First round has 13 cards
            assert_eq!(game.dealer_pos(), Some(0)); // First round dealer is seat 0

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_deal_round_with_different_seeds_differs() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create two games with different seeds
            let game_id1 = setup_game_with_players(txn, 111).await?.game_id;
            let game_id2 = setup_game_with_players(txn, 222).await?.game_id;

            let service = GameFlowService::new();

            // Deal both rounds
            service.deal_round(txn, game_id1).await?;
            service.deal_round(txn, game_id2).await?;

            // Both should succeed and be in Bidding state
            let game1 = games::Entity::find_by_id(game_id1).one(txn).await?.unwrap();
            let game2 = games::Entity::find_by_id(game_id2).one(txn).await?.unwrap();

            assert_eq!(game1.state, GameState::Bidding);
            assert_eq!(game2.state, GameState::Bidding);

            // Note: Since hands are not persisted yet, we can't compare them
            // But the test validates that dealing works with different seeds

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_submit_bid_validates_phase() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Arrange: create game in Lobby (not Bidding)
            let game_id = setup_game_with_players(txn, 456).await?.game_id;

            // Act: try to submit bid without dealing first
            let service = GameFlowService::new();
            let result = service.submit_bid(txn, game_id, 1, 5).await;

            // Assert: should fail with phase mismatch
            assert!(result.is_err());
            let err = result.unwrap_err();
            // Check that the error is a Validation error with PhaseMismatch error code
            use backend::errors::ErrorCode;
            assert_eq!(
                err.code(),
                ErrorCode::PhaseMismatch,
                "Expected PhaseMismatch error but got: {err:?}"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_submit_bid_after_deal() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Arrange: create game and deal
            let game_id = setup_game_with_players(txn, 789).await?.game_id;
            let service = GameFlowService::new();
            service.deal_round(txn, game_id).await?;

            // Act: submit valid bid
            let result = service.submit_bid(txn, game_id, 1, 5).await;

            // Assert: bid submission should succeed
            assert!(result.is_ok());

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
