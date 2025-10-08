mod common;
mod support;

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::entities::{game_players, games, users};
use backend::error::AppError;
use backend::errors::ErrorCode;
use backend::infra::state::build_state;
use backend::services::players::PlayerService;
use sea_orm::{ActiveModelTrait, Set};

#[tokio::test]
async fn test_get_display_name_by_seat_success() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data
            let game_id = create_test_game(txn).await?;
            let user_id = create_test_user(txn, "alice", Some("Alice")).await?;
            create_test_game_player(txn, game_id, user_id, 0).await?;

            // Test the service
            let service = PlayerService::new();
            let result = service.get_display_name_by_seat(txn, game_id, 0).await?;

            assert_eq!(result, "Alice");
            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_get_display_name_by_seat_invalid_seat() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Test the service with invalid seat (no DB data needed for validation)
            let service = PlayerService::new();
            let result = service.get_display_name_by_seat(txn, 1, 5).await;

            match result {
                Err(domain_err) => {
                    let err: AppError = domain_err.into();
                    match err {
                        AppError::Validation { code, detail, .. } => {
                            assert_eq!(code, ErrorCode::InvalidSeat);
                            assert!(detail.contains("Seat must be between 0 and 3"));
                        }
                        _ => panic!("Expected Validation error for invalid seat"),
                    }
                }
                _ => panic!("Expected Validation error for invalid seat"),
            }
            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_get_display_name_by_seat_player_not_found() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test game but no players
            let game_id = create_test_game(txn).await?;

            // Test the service
            let service = PlayerService::new();
            let result = service.get_display_name_by_seat(txn, game_id, 0).await;

            match result {
                Err(domain_err) => {
                    let err: AppError = domain_err.into();
                    match err {
                        AppError::NotFound { code, detail, .. } => {
                            assert_eq!(code, ErrorCode::PlayerNotFound);
                            assert!(detail.contains("Player not found at seat"));
                        }
                        _ => panic!("Expected NotFound error but got: {err:?}"),
                    }
                }
                Ok(display_name) => {
                    panic!("Expected error but got success: {display_name}");
                }
            }
            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_get_display_name_by_seat_fallback_to_sub() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data with no username (should fall back to sub)
            let game_id = create_test_game(txn).await?;
            let user_id = create_test_user(txn, "bob", None).await?;
            create_test_game_player(txn, game_id, user_id, 1).await?;

            // Test the service
            let service = PlayerService::new();
            let result = service.get_display_name_by_seat(txn, game_id, 1).await?;

            assert_eq!(result, "bob");
            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_get_display_name_by_seat_multiple_seats() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data with multiple players
            let game_id = create_test_game(txn).await?;
            let user1_id = create_test_user(txn, "alice", Some("Alice")).await?;
            let user2_id = create_test_user(txn, "bob", Some("Bob")).await?;
            let user3_id = create_test_user(txn, "charlie", Some("Charlie")).await?;

            create_test_game_player(txn, game_id, user1_id, 0).await?;
            create_test_game_player(txn, game_id, user2_id, 1).await?;
            create_test_game_player(txn, game_id, user3_id, 2).await?;

            // Test the service for different seats
            let service = PlayerService::new();

            let result0 = service.get_display_name_by_seat(txn, game_id, 0).await?;
            assert_eq!(result0, "Alice");

            let result1 = service.get_display_name_by_seat(txn, game_id, 1).await?;
            assert_eq!(result1, "Bob");

            let result2 = service.get_display_name_by_seat(txn, game_id, 2).await?;
            assert_eq!(result2, "Charlie");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

// Helper functions for test data creation

async fn create_test_game(txn: &impl sea_orm::ConnectionTrait) -> Result<i64, AppError> {
    // Create a user first to use as created_by
    let user_id = create_test_user(txn, "creator", Some("Creator")).await?;

    let game = games::ActiveModel {
        id: sea_orm::NotSet,
        created_by: Set(Some(user_id)),
        visibility: Set(games::GameVisibility::Public),
        state: Set(games::GameState::Lobby),
        created_at: Set(time::OffsetDateTime::now_utc()),
        updated_at: Set(time::OffsetDateTime::now_utc()),
        started_at: Set(None),
        ended_at: Set(None),
        name: Set(Some("Test Game".to_string())),
        join_code: Set(Some("ABC123".to_string())),
        rules_version: Set("1.0".to_string()),
        rng_seed: Set(Some(12345)),
        current_round: Set(Some(1)),
        hand_size: Set(Some(13)),
        dealer_pos: Set(Some(0)),
        lock_version: Set(1),
    };

    let inserted = game.insert(txn).await?;
    Ok(inserted.id)
}

async fn create_test_user(
    txn: &impl sea_orm::ConnectionTrait,
    sub: &str,
    username: Option<&str>,
) -> Result<i64, AppError> {
    let user = users::ActiveModel {
        id: sea_orm::NotSet,
        sub: Set(sub.to_string()),
        username: Set(username.map(|s| s.to_string())),
        is_ai: Set(false),
        created_at: Set(time::OffsetDateTime::now_utc()),
        updated_at: Set(time::OffsetDateTime::now_utc()),
    };

    let inserted = user.insert(txn).await?;
    Ok(inserted.id)
}

async fn create_test_game_player(
    txn: &impl sea_orm::ConnectionTrait,
    game_id: i64,
    user_id: i64,
    turn_order: i32,
) -> Result<(), AppError> {
    let game_player = game_players::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(game_id),
        user_id: Set(user_id),
        turn_order: Set(turn_order),
        is_ready: Set(false),
        created_at: Set(time::OffsetDateTime::now_utc()),
    };

    game_player.insert(txn).await?;
    Ok(())
}
