mod common;
mod support;

use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::services::game_flow::GameFlowService;
use support::game_phases::setup_game_in_bidding_phase;

/// Test: Dealer cannot bid when sum equals hand_size
#[tokio::test]
async fn test_dealer_bid_restriction_rejects_exact_sum() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Set up game in Bidding phase (hand_size = 13, dealer = 0)
            let setup = setup_game_in_bidding_phase(txn, 12345).await?;
            let service = GameFlowService::new();

            // Bidding starts at dealer + 1 = seat 1
            // First 3 non-dealer players bid: 5 + 4 + 3 = 12
            service.submit_bid(txn, setup.game_id, 1, 5).await?;
            service.submit_bid(txn, setup.game_id, 2, 4).await?;
            service.submit_bid(txn, setup.game_id, 3, 3).await?;

            // Dealer (seat 0) tries to bid 1, which would make sum = 13 (not allowed)
            let result = service.submit_bid(txn, setup.game_id, 0, 1).await;

            assert!(result.is_err(), "Dealer bid creating exact sum should fail");

            // Verify it's an InvalidBid error
            match result.unwrap_err() {
                AppError::Validation { code, .. } => {
                    assert_eq!(code.as_str(), "INVALID_BID");
                }
                e => panic!("Expected Validation error with InvalidBid, got {e:?}"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Dealer CAN bid when sum != hand_size
#[tokio::test]
async fn test_dealer_bid_restriction_allows_other_values() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Set up game in Bidding phase (hand_size = 13, dealer = 0)
            let setup = setup_game_in_bidding_phase(txn, 12346).await?;
            let service = GameFlowService::new();

            // Bidding starts at dealer + 1 = seat 1
            // First 3 non-dealer players bid: 5 + 4 + 3 = 12
            service.submit_bid(txn, setup.game_id, 1, 5).await?;
            service.submit_bid(txn, setup.game_id, 2, 4).await?;
            service.submit_bid(txn, setup.game_id, 3, 3).await?;

            // Dealer (seat 0) bids 0 (sum = 12, OK) or 2 (sum = 14, OK)
            let result = service.submit_bid(txn, setup.game_id, 0, 0).await;
            assert!(
                result.is_ok(),
                "Dealer bid with sum < hand_size should succeed"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Dealer bid restriction only applies to 4th bid
#[tokio::test]
async fn test_dealer_bid_restriction_only_for_dealer() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Set up game in Bidding phase (hand_size = 13, dealer = 0)
            let setup = setup_game_in_bidding_phase(txn, 12347).await?;
            let service = GameFlowService::new();

            // Bidding starts at dealer + 1 = seat 1
            // First 3 non-dealer players can bid any valid value
            service.submit_bid(txn, setup.game_id, 1, 13).await?; // Max bid OK for non-dealer
            service.submit_bid(txn, setup.game_id, 2, 0).await?;
            service.submit_bid(txn, setup.game_id, 3, 0).await?;

            // Dealer (seat 0) must avoid bid that sums to 13
            // sum = 13 + 0 + 0 + X, so dealer cannot bid 0
            let result = service.submit_bid(txn, setup.game_id, 0, 0).await;
            assert!(result.is_err(), "Dealer bid with exact sum should fail");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Dealer bid restriction in small hand
#[tokio::test]
async fn test_dealer_bid_restriction_small_hand() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            use backend::adapters::games_sea::{GameUpdateRound, GameUpdateState};
            use backend::entities::games::GameState as DbGameState;
            use backend::repos::rounds;
            use support::game_setup::setup_game_with_players;

            // Create game with players
            let game_setup = setup_game_with_players(txn, 12348).await?;

            // Manually create a round with hand_size = 2
            // Round 13 has hand_size 2, and with starting_dealer=0, dealer_pos=(0+13-1)%4=0
            let _round = rounds::create_round(txn, game_setup.game_id, 13, 2, 0).await?;

            // Update game to current_round = 13 and set starting_dealer_pos
            let update_state = GameUpdateState::new(game_setup.game_id, DbGameState::Bidding, 1);
            let updated = backend::adapters::games_sea::update_state(txn, update_state).await?;

            let update_round = GameUpdateRound::new(game_setup.game_id, updated.lock_version)
                .with_current_round(13)
                .with_starting_dealer_pos(0); // Set starting dealer to seat 0, so round 13 dealer = 0
            backend::adapters::games_sea::update_round(txn, update_round).await?;

            let service = GameFlowService::new();

            // Bidding starts at dealer + 1 = seat 1
            // Bids: 0 + 1 + 0 = 1, dealer cannot bid 1 (sum would be 2)
            service.submit_bid(txn, game_setup.game_id, 1, 0).await?;
            service.submit_bid(txn, game_setup.game_id, 2, 1).await?;
            service.submit_bid(txn, game_setup.game_id, 3, 0).await?;

            let result = service.submit_bid(txn, game_setup.game_id, 0, 1).await;
            assert!(result.is_err(), "Dealer bid creating sum=2 should fail");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
