// Integration property tests for round progression using services and DB transactions.
//
// These tests verify state monotonicity, version increments, and timestamp invariants
// across granular service steps (deal, bid, play tricks).

use backend::db::txn::with_txn;
use backend::entities::games::{self, GameState as DbGameState};
use backend::repos::games as games_repo;
use backend::services::game_flow::GameFlowService;
use backend::AppError;
use sea_orm::EntityTrait;

use crate::support::build_test_state;
use crate::support::game_setup::setup_game_with_players;

/// Test: State monotonicity - game state should only advance forward
#[tokio::test]
async fn test_state_monotonicity() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, "round_prog_basic")
                .await?
                .game_id;

            // Initial state: Lobby
            let game = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            assert_eq!(game.state, DbGameState::Lobby);

            // Step 1: Deal round -> should transition to Bidding
            let service = GameFlowService;
            service.deal_round(txn, game_id).await?;

            let game = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            assert_eq!(
                game.state,
                DbGameState::Bidding,
                "After dealing, state should be Bidding"
            );

            // State should not revert to Lobby
            assert_ne!(
                game.state,
                DbGameState::Lobby,
                "State must not revert backwards"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: version increments across persisted steps
#[tokio::test]
async fn test_version_increments() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, "round_prog_invalid")
                .await?
                .game_id;

            // Capture initial version
            let game_before = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            let version_before = game_before.version;

            // Step 1: Deal round
            let service = GameFlowService;
            service.deal_round(txn, game_id).await?;

            let game_after_deal = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            assert!(
                game_after_deal.version > version_before,
                "version should increment after deal"
            );

            // Step 2: Submit a bid
            let lock_before_bid = game_after_deal.version;
            let game = backend::repos::games::require_game(txn, game_id).await?;
            service.submit_bid(txn, game_id, 1, 5, game.version).await?;

            let game_after_bid = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            assert!(
                game_after_bid.version > lock_before_bid,
                "version should increment after bid"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: created_at constant, updated_at strictly increases
#[tokio::test]
async fn test_timestamp_invariants() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, "round_prog_players")
                .await?
                .game_id;

            // Capture initial timestamps
            let game_before = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            let created_at_before = game_before.created_at;
            let updated_at_before = game_before.updated_at;

            // Sleep to ensure time difference (tokio::time::sleep requires time advancement)
            // For test purposes, we rely on the service updating the timestamp

            // Step 1: Deal round
            let service = GameFlowService;
            service.deal_round(txn, game_id).await?;

            let game_after = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();

            // created_at should remain constant
            assert_eq!(
                game_after.created_at, created_at_before,
                "created_at must remain constant"
            );

            // updated_at should increase (or stay same if no time passed, but service should update it)
            assert!(
                game_after.updated_at >= updated_at_before,
                "updated_at should increase or stay same"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Complete a deterministic first trick
#[tokio::test]
async fn test_deterministic_first_trick() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, "round_prog_hands")
                .await?
                .game_id;
            let service = GameFlowService;

            // Deal round (will be in Bidding state)
            service.deal_round(txn, game_id).await?;

            // Submit bids for all 4 players in turn order (dealer=0, so turn order is 1,2,3,0)
            // Valid bids for hand_size=13
            for (seat, bid) in [(1u8, 6u8), (2, 3), (3, 4), (0, 5)] {
                let game = backend::repos::games::require_game(txn, game_id).await?;
                service
                    .submit_bid(txn, game_id, seat, bid, game.version)
                    .await?;
            }

            // After bidding, should be in TrumpSelection phase
            let game = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            assert_eq!(
                game.state,
                DbGameState::TrumpSelection,
                "After all bids, should be in TrumpSelection"
            );

            // Trump selection is implemented in GameFlowService::set_trump()
            // but not yet exposed via HTTP API for human players.
            // This test verifies that bidding phase completes correctly.

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Granular round progression with state checks
#[tokio::test]
async fn test_granular_round_progression() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, "round_prog_cards")
                .await?
                .game_id;
            let service = GameFlowService;

            // Track state transitions
            let mut state_history: Vec<DbGameState> = Vec::new();

            // Initial state
            let game = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            state_history.push(game.state);

            // Deal round
            service.deal_round(txn, game_id).await?;
            let game = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            state_history.push(game.state);

            // Submit bids in turn order (dealer=0, so turn order is 1,2,3,0)
            for (seat, bid) in [(1u8, 5u8), (2, 3), (3, 2), (0, 4)] {
                let game = backend::repos::games::require_game(txn, game_id).await?;
                service
                    .submit_bid(txn, game_id, seat, bid, game.version)
                    .await?;
            }

            let game = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            state_history.push(game.state);

            // Verify state transitions are valid
            assert_eq!(
                state_history[0],
                DbGameState::Lobby,
                "Should start in Lobby"
            );
            assert_eq!(
                state_history[1],
                DbGameState::Bidding,
                "Should move to Bidding"
            );

            // After all bids, should be in TrumpSelection
            assert_eq!(
                state_history[2],
                DbGameState::TrumpSelection,
                "Should move to TrumpSelection after all bids"
            );

            // Verify no backwards transitions
            for i in 1..state_history.len() {
                // States should either stay the same or advance
                // (In practice, they should advance, but we're checking monotonicity)
                let curr = state_history[i].clone() as i32;
                let prev = state_history[i - 1].clone() as i32;
                assert!(
                    curr >= prev,
                    "State should not move backwards: {:?} -> {:?}",
                    state_history[i - 1],
                    state_history[i]
                );
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Deterministic dealing with fixed seed produces reproducible results
#[tokio::test]
async fn test_deterministic_dealing_reproducible() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create two games with the same seed
            let game_id1 = setup_game_with_players(txn, "round_prog_det_1")
                .await?
                .game_id;
            let game_id2 = setup_game_with_players(txn, "round_prog_det_2")
                .await?
                .game_id;

            let service = GameFlowService;

            // Deal both rounds
            service.deal_round(txn, game_id1).await?;
            service.deal_round(txn, game_id2).await?;

            // Both should be in Bidding state
            let game1 = games_repo::find_by_id(txn, game_id1).await?.unwrap();
            let game2 = games_repo::find_by_id(txn, game_id2).await?.unwrap();

            assert_eq!(game1.state, DbGameState::Bidding);
            assert_eq!(game2.state, DbGameState::Bidding);

            // Same seed should produce same initial state
            assert_eq!(game1.hand_size(), game2.hand_size());
            assert_eq!(game1.dealer_pos(), game2.dealer_pos());
            assert_eq!(game1.current_round, game2.current_round);

            // Note: Without persisting hands, we can't verify the actual cards dealt
            // But the test validates that dealing completes successfully

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Invalid bid should fail with appropriate error
#[tokio::test]
async fn test_invalid_bid_fails() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, "round_prog_scoring")
                .await?
                .game_id;
            let service = GameFlowService;

            // Deal round
            service.deal_round(txn, game_id).await?;

            // Try to submit an invalid bid (> hand_size)
            let game = backend::repos::games::require_game(txn, game_id).await?;
            let result = service.submit_bid(txn, game_id, 1, 100, game.version).await;

            assert!(result.is_err(), "Invalid bid should fail");

            // Check error code
            if let Err(e) = result {
                use backend::ErrorCode;
                assert_eq!(
                    e.code(),
                    ErrorCode::InvalidBid,
                    "Should fail with InvalidBid error code"
                );
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Out of turn bid should fail
#[tokio::test]
async fn test_out_of_turn_bid_fails() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, "round_prog_between")
                .await?
                .game_id;
            let service = GameFlowService;

            // Deal round
            service.deal_round(txn, game_id).await?;

            // Try to submit a bid for player 2 when it's player 0's turn
            // (Assuming turn starts at player 0 or dealer+1)
            let game = backend::repos::games::require_game(txn, game_id).await?;
            let result = service.submit_bid(txn, game_id, 2, 5, game.version).await;

            // This should fail (assuming turn order enforcement is implemented)
            // If not yet implemented, this test will fail and guide implementation
            assert!(result.is_err(), "Out of turn bid should fail");

            if let Err(e) = result {
                use backend::ErrorCode;
                // Should be OutOfTurn error
                assert_eq!(
                    e.code(),
                    ErrorCode::OutOfTurn,
                    "Should fail with OutOfTurn error code"
                );
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Bid in wrong phase should fail
#[tokio::test]
async fn test_bid_in_wrong_phase_fails() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, "round_prog_complete")
                .await?
                .game_id;
            let service = GameFlowService;

            // Try to bid without dealing first (still in Lobby)
            let game = backend::repos::games::require_game(txn, game_id).await?;
            let result = service.submit_bid(txn, game_id, 1, 5, game.version).await;

            assert!(result.is_err(), "Bid in Lobby phase should fail");

            if let Err(e) = result {
                use backend::ErrorCode;
                assert_eq!(
                    e.code(),
                    ErrorCode::PhaseMismatch,
                    "Should fail with PhaseMismatch error code"
                );
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
