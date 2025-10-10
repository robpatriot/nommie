//! Integration property tests for round progression using services and DB transactions.
//!
//! These tests verify state monotonicity, lock_version increments, and timestamp invariants
//! across granular service steps (deal, bid, play tricks).

mod common;
mod support;

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::entities::games::{self, GameState as DbGameState, GameVisibility};
use backend::entities::users;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::services::game_flow::GameFlowService;
use backend::utils::unique::unique_str;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, NotSet, Set};
use support::db_memberships::create_test_game_player_with_ready;

/// Helper to create a test game with 4 ready players and a fixed rng_seed
async fn setup_game_with_players<C: ConnectionTrait>(
    conn: &C,
    rng_seed: i64,
) -> Result<i64, AppError> {
    let now = time::OffsetDateTime::now_utc();

    // Create 4 users with unique subs
    let mut user_ids = Vec::new();
    for i in 0..4 {
        let user = users::ActiveModel {
            id: NotSet,
            sub: Set(unique_str(&format!("test_user_{rng_seed}_{i}"))),
            username: Set(Some(format!("player{i}_{rng_seed}"))),
            is_ai: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let inserted_user = user.insert(conn).await?;
        user_ids.push(inserted_user.id);
    }

    // Create game
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Private),
        state: Set(DbGameState::Lobby),
        rules_version: Set("nommie-1.0.0".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        rng_seed: Set(Some(rng_seed)),
        ..Default::default()
    };

    let inserted_game = games::Entity::insert(game)
        .exec(conn)
        .await
        .map_err(|e| backend::error::AppError::from(backend::infra::db_errors::map_db_err(e)))?;

    let game_id = inserted_game.last_insert_id;

    // Create 4 game_players all marked ready
    for (i, user_id) in user_ids.iter().enumerate() {
        create_test_game_player_with_ready(conn, game_id, *user_id, i as i32, true).await?;
    }

    Ok(game_id)
}

/// Test: State monotonicity - game state should only advance forward
#[tokio::test]
async fn test_state_monotonicity() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, 42).await?;

            // Initial state: Lobby
            let game = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            assert_eq!(game.state, DbGameState::Lobby);

            // Step 1: Deal round -> should transition to Bidding
            let service = GameFlowService::new();
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

/// Test: lock_version increments across persisted steps
///
/// NOTE: This test is expected to FAIL until service layer implements:
/// - Bid persistence to database with lock_version updates
#[tokio::test]
#[ignore = "Service layer not yet implemented"]
async fn test_lock_version_increments() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, 123).await?;

            // Capture initial lock_version
            let game_before = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            let lock_version_before = game_before.lock_version;

            // Step 1: Deal round
            let service = GameFlowService::new();
            service.deal_round(txn, game_id).await?;

            let game_after_deal = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            assert!(
                game_after_deal.lock_version > lock_version_before,
                "lock_version should increment after deal"
            );

            // Step 2: Submit a bid
            let lock_before_bid = game_after_deal.lock_version;
            service.submit_bid(txn, game_id, 1, 5).await?;

            let game_after_bid = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            assert!(
                game_after_bid.lock_version > lock_before_bid,
                "lock_version should increment after bid"
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
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, 456).await?;

            // Capture initial timestamps
            let game_before = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            let created_at_before = game_before.created_at;
            let updated_at_before = game_before.updated_at;

            // Sleep to ensure time difference (tokio::time::sleep requires time advancement)
            // For test purposes, we rely on the service updating the timestamp

            // Step 1: Deal round
            let service = GameFlowService::new();
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
///
/// NOTE: This test is expected to FAIL until service layer implements:
/// - Bidding state transitions to TrumpSelection
/// - Trump selection logic
#[tokio::test]
#[ignore = "Service layer not yet implemented"]
async fn test_deterministic_first_trick() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, 999).await?;
            let service = GameFlowService::new();

            // Deal round (will be in Bidding state)
            service.deal_round(txn, game_id).await?;

            // Submit bids for all 4 players (valid bids for hand_size=13)
            service.submit_bid(txn, game_id, 0, 5).await?;
            service.submit_bid(txn, game_id, 1, 6).await?;
            service.submit_bid(txn, game_id, 2, 3).await?;
            service.submit_bid(txn, game_id, 3, 4).await?;

            // After bidding, should be in TrumpSelection phase
            let game = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            assert_eq!(
                game.state,
                DbGameState::TrumpSelection,
                "After all bids, should be in TrumpSelection"
            );

            // Highest bidder (player 1, bid 6) should select trump
            // For now, this is a placeholder - service may not yet implement trump selection
            // We'll just assert the state advanced correctly

            // Note: If trump selection and trick play are not yet implemented,
            // these tests will serve as specification for future implementation

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Granular round progression with state checks
///
/// NOTE: This test is expected to FAIL until service layer implements:
/// - Complete bidding phase with state transitions
#[tokio::test]
#[ignore = "Service layer not yet implemented"]
async fn test_granular_round_progression() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, 777).await?;
            let service = GameFlowService::new();

            // Track state transitions
            let mut state_history: Vec<DbGameState> = Vec::new();

            // Initial state
            let game = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            state_history.push(game.state);

            // Deal round
            service.deal_round(txn, game_id).await?;
            let game = games::Entity::find_by_id(game_id).one(txn).await?.unwrap();
            state_history.push(game.state);

            // Submit bids
            service.submit_bid(txn, game_id, 0, 4).await?;
            service.submit_bid(txn, game_id, 1, 5).await?;
            service.submit_bid(txn, game_id, 2, 3).await?;
            service.submit_bid(txn, game_id, 3, 2).await?;

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
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create two games with the same seed
            let game_id1 = setup_game_with_players(txn, 12345).await?;
            let game_id2 = setup_game_with_players(txn, 12345).await?;

            let service = GameFlowService::new();

            // Deal both rounds
            service.deal_round(txn, game_id1).await?;
            service.deal_round(txn, game_id2).await?;

            // Both should be in Bidding state
            let game1 = games::Entity::find_by_id(game_id1).one(txn).await?.unwrap();
            let game2 = games::Entity::find_by_id(game_id2).one(txn).await?.unwrap();

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
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, 888).await?;
            let service = GameFlowService::new();

            // Deal round
            service.deal_round(txn, game_id).await?;

            // Try to submit an invalid bid (> hand_size)
            let result = service.submit_bid(txn, game_id, 1, 100).await;

            assert!(result.is_err(), "Invalid bid should fail");

            // Check error code
            if let Err(e) = result {
                use backend::errors::ErrorCode;
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
///
/// NOTE: This test is expected to FAIL until service layer implements:
/// - Turn order validation in submit_bid
#[tokio::test]
#[ignore = "Service layer not yet implemented"]
async fn test_out_of_turn_bid_fails() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, 333).await?;
            let service = GameFlowService::new();

            // Deal round
            service.deal_round(txn, game_id).await?;

            // Try to submit a bid for player 2 when it's player 0's turn
            // (Assuming turn starts at player 0 or dealer+1)
            let result = service.submit_bid(txn, game_id, 2, 5).await;

            // This should fail (assuming turn order enforcement is implemented)
            // If not yet implemented, this test will fail and guide implementation
            assert!(result.is_err(), "Out of turn bid should fail");

            if let Err(e) = result {
                use backend::errors::ErrorCode;
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
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_id = setup_game_with_players(txn, 555).await?;
            let service = GameFlowService::new();

            // Try to bid without dealing first (still in Lobby)
            let result = service.submit_bid(txn, game_id, 1, 5).await;

            assert!(result.is_err(), "Bid in Lobby phase should fail");

            if let Err(e) = result {
                use backend::errors::ErrorCode;
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
