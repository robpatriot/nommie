mod common;
mod support;

use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::error::AppError;
use backend::errors::ErrorCode;
use backend::infra::state::build_state;
use backend::services::game_flow::GameFlowService;

/// Test that verifies the game prevents playing the same card twice.
///
/// This test:
/// 1. Sets up a game with 4 players in the Playing phase
/// 2. Plays a specific card in trick 1
/// 3. Attempts to play the same card in trick 2
/// 4. Asserts that the second play fails with CardNotInHand error
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

    // Set up game with 4 players (not ready yet, so we can start the game properly)
    let options = support::game_setup::GameSetupOptions::default()
        .with_rng_seed(12345)
        .with_ready(false);
    let setup = support::game_setup::setup_game_with_options(txn, options).await?;
    let game_id = setup.game_id;

    let service = GameFlowService::new();

    // Start the game
    for user_id in &setup.user_ids {
        service.mark_ready(txn, game_id, *user_id).await?;
    }

    // Get current round and player hands from database
    let game = backend::adapters::games_sea::require_game(txn, game_id).await?;
    let round_no = game.current_round.expect("Game should have started");
    let round = backend::repos::rounds::find_by_game_and_round(txn, game_id, round_no)
        .await?
        .expect("Round should exist");

    // Get player 0's hand and pick the first card
    let hand = backend::repos::hands::find_by_round_and_seat(txn, round.id, 0)
        .await?
        .expect("Player 0 should have a hand");
    let first_card = backend::domain::cards_parsing::from_stored_format(
        &hand.cards[0].suit,
        &hand.cards[0].rank,
    )?;

    // Find dealer position to determine bidding order
    let game = backend::adapters::games_sea::require_game(txn, game_id).await?;
    let dealer = game
        .starting_dealer_pos
        .expect("Dealer position should be set");

    // Complete bidding phase - bidding starts at dealer+1 and goes clockwise
    // Bid 1 for first 3 players, then last player (dealer) bids 0 to avoid sum = hand_size
    for i in 0..4 {
        let seat = (dealer + 1 + i) % 4;
        let bid_value = if i < 3 { 1 } else { 0 };
        service.submit_bid(txn, game_id, seat, bid_value).await?;
    }

    // Select trump - the highest bidder (one of the first 3) selects trump
    // The first bidder among those who bid 1 wins
    let trump_selector = (dealer + 1) % 4;
    service
        .set_trump(
            txn,
            game_id,
            trump_selector,
            backend::repos::rounds::Trump::Hearts,
        )
        .await?;

    // Play first trick - player at seat 0 plays the first card
    service.play_card(txn, game_id, 0, first_card).await?;

    // Complete the first trick with other players by picking valid cards from their hands
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

    // Now try to play the same card again in the next trick
    // This should fail with a validation error for CardNotInHand
    let result = service.play_card(txn, game_id, 0, first_card).await;

    match result {
        Err(AppError::Validation {
            code: ErrorCode::CardNotInHand,
            ..
        }) => {
            // Expected - test passes
        }
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
