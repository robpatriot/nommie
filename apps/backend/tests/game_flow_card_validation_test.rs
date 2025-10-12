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

    // Create 4 test users
    let user1 = support::factory::create_test_user(txn, "player1", Some("Player 1")).await?;
    let user2 = support::factory::create_test_user(txn, "player2", Some("Player 2")).await?;
    let user3 = support::factory::create_test_user(txn, "player3", Some("Player 3")).await?;
    let user4 = support::factory::create_test_user(txn, "player4", Some("Player 4")).await?;

    // Create a game with a deterministic seed
    use backend::entities::games::{self, GameState as DbGameState, GameVisibility};
    use sea_orm::{ActiveModelTrait, NotSet, Set};
    use time::OffsetDateTime;

    let now = OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        id: NotSet,
        created_by: Set(Some(user1)),
        visibility: Set(GameVisibility::Public),
        state: Set(DbGameState::Lobby),
        created_at: Set(now),
        updated_at: Set(now),
        started_at: Set(None),
        ended_at: Set(None),
        name: Set(Some("Duplicate Card Test".to_string())),
        join_code: Set(Some(format!("TEST{}", rand::random::<u32>() % 1000000))),
        rules_version: Set("1.0".to_string()),
        rng_seed: Set(Some(12345)),
        current_round: Set(None),
        starting_dealer_pos: Set(None),
        current_trick_no: Set(0),
        current_round_id: Set(None),
        lock_version: Set(0),
    };
    let game_id = game.insert(txn).await?.id;

    // Add all players
    use backend::repos::memberships;
    memberships::create_membership(txn, game_id, user1, 0, false, memberships::GameRole::Player)
        .await?;
    memberships::create_membership(txn, game_id, user2, 1, false, memberships::GameRole::Player)
        .await?;
    memberships::create_membership(txn, game_id, user3, 2, false, memberships::GameRole::Player)
        .await?;
    memberships::create_membership(txn, game_id, user4, 3, false, memberships::GameRole::Player)
        .await?;

    let service = GameFlowService::new();

    // Start the game
    service.mark_ready(txn, game_id, user1).await?;
    service.mark_ready(txn, game_id, user2).await?;
    service.mark_ready(txn, game_id, user3).await?;
    service.mark_ready(txn, game_id, user4).await?;

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
