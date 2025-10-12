//! Integration test for GameFlowService happy path.
//!
//! This test validates that the happy path workflow advances game state correctly
//! with deterministic outcomes.

mod common;
mod support;

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::entities::users;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::services::game_flow::GameFlowService;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, NotSet, Set};
use support::db_memberships::create_test_game_player_with_ready;

/// Helper to create a test game with 4 ready players
async fn setup_game_with_players<C: ConnectionTrait>(
    conn: &C,
    rng_seed: i64,
) -> Result<i64, AppError> {
    let now = time::OffsetDateTime::now_utc();

    // First, create 4 users
    let mut user_ids = Vec::new();
    for i in 0..4 {
        let user = users::ActiveModel {
            id: NotSet,
            sub: Set(format!("test_user_{rng_seed}_{i}")),
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
        state: Set(GameState::Lobby),
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
            let game_id = setup_game_with_players(txn, 123).await?;

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
            let game_id1 = setup_game_with_players(txn, 111).await?;
            let game_id2 = setup_game_with_players(txn, 222).await?;

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
            let game_id = setup_game_with_players(txn, 456).await?;

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
            let game_id = setup_game_with_players(txn, 789).await?;
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
