//! Integration tests for game flow validations.
//!
//! This module tests validation rules and error cases throughout the game flow:
//! - Phase requirements
//! - Dealer bid restrictions
//! - Trump selection rules
//! - Card play constraints

mod common;
mod support;

use backend::adapters::games_sea;
use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::{with_txn, SharedTxn};
use backend::error::AppError;
use backend::errors::ErrorCode;
use backend::infra::state::build_state;
use backend::repos::rounds;
use backend::services::game_flow::GameFlowService;
use support::game_phases::{setup_game_in_bidding_phase, setup_game_in_trump_selection_phase};
use support::game_setup::{setup_game_with_players, GameSetupOptions};

// ============================================================================
// Phase Validation Tests
// ============================================================================

#[tokio::test]
async fn test_submit_bid_rejects_wrong_phase() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, 456).await?.game_id;

            let service = GameFlowService::new();
            let result = service.submit_bid(txn, game_id, 1, 5).await;

            assert!(result.is_err());
            let err = result.unwrap_err();
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

// ============================================================================
// Dealer Bid Restriction Tests
// ============================================================================

#[tokio::test]
async fn test_dealer_bid_restriction_rejects_exact_sum() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
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

#[tokio::test]
async fn test_dealer_bid_restriction_allows_non_exact_sum() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let setup = setup_game_in_bidding_phase(txn, 12346).await?;
            let service = GameFlowService::new();

            // First 3 non-dealer players bid: 5 + 4 + 3 = 12
            service.submit_bid(txn, setup.game_id, 1, 5).await?;
            service.submit_bid(txn, setup.game_id, 2, 4).await?;
            service.submit_bid(txn, setup.game_id, 3, 3).await?;

            // Dealer (seat 0) bids 0 (sum = 12, OK)
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

#[tokio::test]
async fn test_dealer_bid_restriction_only_applies_to_dealer() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let setup = setup_game_in_bidding_phase(txn, 12347).await?;
            let service = GameFlowService::new();

            // Non-dealer players can bid any valid value
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

#[tokio::test]
async fn test_dealer_bid_restriction_in_small_hand() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            use backend::adapters::games_sea::{GameUpdateRound, GameUpdateState};
            use backend::entities::games::GameState as DbGameState;

            let game_setup = setup_game_with_players(txn, 12348).await?;

            // Manually create a round with hand_size = 2
            // Round 13 has hand_size 2, and with starting_dealer=0, dealer_pos=(0+13-1)%4=0
            let _round = rounds::create_round(txn, game_setup.game_id, 13, 2, 0).await?;

            let update_state = GameUpdateState::new(game_setup.game_id, DbGameState::Bidding, 1);
            let updated = backend::adapters::games_sea::update_state(txn, update_state).await?;

            let update_round = GameUpdateRound::new(game_setup.game_id, updated.lock_version)
                .with_current_round(13)
                .with_starting_dealer_pos(0);
            backend::adapters::games_sea::update_round(txn, update_round).await?;

            let service = GameFlowService::new();

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

// ============================================================================
// Trump Selection Validation Tests
// ============================================================================

#[tokio::test]
async fn test_only_bid_winner_can_choose_trump() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Round 1: dealer at seat 0, bids: 2, 3, 4, 3 = 12
            let setup = setup_game_in_trump_selection_phase(txn, 12345, [2, 3, 4, 3]).await?;
            let service = GameFlowService::new();

            let game_after_bids = games_sea::find_by_id(txn, setup.game_id).await?.unwrap();
            assert_eq!(
                game_after_bids.state,
                backend::entities::games::GameState::TrumpSelection
            );

            let result = service
                .set_trump(txn, setup.game_id, 0, rounds::Trump::Hearts)
                .await;

            assert!(result.is_err());
            let err = result.unwrap_err();
            let error_message = format!("{err}");
            assert!(
                error_message.contains("Only the winning bidder")
                    || error_message.contains("Out of turn"),
                "Expected OutOfTurn/bid winner error, got: {error_message}"
            );

            let result = service
                .set_trump(txn, setup.game_id, 1, rounds::Trump::Spades)
                .await;
            assert!(result.is_err());

            let result = service
                .set_trump(txn, setup.game_id, 2, rounds::Trump::Diamonds)
                .await;
            assert!(result.is_ok(), "Winning bidder should be able to set trump");

            let round = rounds::find_by_game_and_round(txn, setup.game_id, 1)
                .await?
                .unwrap();
            assert_eq!(round.trump, Some(rounds::Trump::Diamonds));

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

#[tokio::test]
async fn test_trump_selection_with_tied_bids() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Round 1: dealer at seat 0, bidding starts at seat 1
            // Bids: 2, 4, 2, 4 - Seats 1 and 3 both bid 4, but seat 1 bid first
            let setup = setup_game_in_trump_selection_phase(txn, 12346, [2, 4, 2, 4]).await?;
            let service = GameFlowService::new();

            let result = service
                .set_trump(txn, setup.game_id, 3, rounds::Trump::Hearts)
                .await;
            assert!(
                result.is_err(),
                "Seat 3 should not be able to set trump despite tied bid"
            );

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

// ============================================================================
// Card Play Validation Tests
// ============================================================================

#[tokio::test]
async fn test_cannot_play_same_card_twice() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    let options = GameSetupOptions::default()
        .with_rng_seed(12345)
        .with_ready(false);
    let setup = support::game_setup::setup_game_with_options(txn, options).await?;
    let game_id = setup.game_id;

    let service = GameFlowService::new();

    for user_id in &setup.user_ids {
        service.mark_ready(txn, game_id, *user_id).await?;
    }

    let game = backend::adapters::games_sea::require_game(txn, game_id).await?;
    let round_no = game.current_round.expect("Game should have started");
    let round = backend::repos::rounds::find_by_game_and_round(txn, game_id, round_no)
        .await?
        .expect("Round should exist");

    let hand = backend::repos::hands::find_by_round_and_seat(txn, round.id, 0)
        .await?
        .expect("Player 0 should have a hand");
    let first_card = backend::domain::cards_parsing::from_stored_format(
        &hand.cards[0].suit,
        &hand.cards[0].rank,
    )?;

    let game = backend::adapters::games_sea::require_game(txn, game_id).await?;
    let dealer = game
        .starting_dealer_pos
        .expect("Dealer position should be set");

    for i in 0..4 {
        let seat = (dealer + 1 + i) % 4;
        let bid_value = if i < 3 { 1 } else { 0 };
        service.submit_bid(txn, game_id, seat, bid_value).await?;
    }

    let trump_selector = (dealer + 1) % 4;
    service
        .set_trump(txn, game_id, trump_selector, rounds::Trump::Hearts)
        .await?;

    service.play_card(txn, game_id, 0, first_card).await?;

    for seat in 1..4 {
        let hand = backend::repos::hands::find_by_round_and_seat(txn, round.id, seat)
            .await?
            .expect("Player should have a hand");
        let card = backend::domain::cards_parsing::from_stored_format(
            &hand.cards[0].suit,
            &hand.cards[0].rank,
        )?;
        service.play_card(txn, game_id, seat, card).await?;
    }

    let result = service.play_card(txn, game_id, 0, first_card).await;

    match result {
        Err(AppError::Validation {
            code: ErrorCode::CardNotInHand,
            ..
        }) => {}
        Ok(_) => {
            panic!("BUG: Game allowed playing the same card twice!");
        }
        Err(other) => {
            panic!("Unexpected error: {other:?}");
        }
    }

    shared.rollback().await?;

    Ok(())
}
