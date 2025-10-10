mod common;
mod support;

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::errors::ErrorCode;
use backend::infra::state::build_state;
use backend::services::players::PlayerService;
use support::db_memberships::create_test_game_player;
use support::factory::{create_test_game, create_test_user};

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
                        AppError::Validation { code, .. } => {
                            assert_eq!(code, ErrorCode::InvalidSeat);
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
                        AppError::NotFound { code, .. } => {
                            assert_eq!(code, ErrorCode::PlayerNotFound);
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
