mod common;
mod support;

use backend::db::txn::with_txn;
use backend::entities::{game_players, games, users};
use backend::error::AppError;
use backend::errors::domain::DomainError;
use backend::infra::state::build_state;
use backend::repos::players;
use sea_orm::{ActiveModelTrait, Set};

#[tokio::test]
async fn test_get_display_name_by_seat_success() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data
            let game_id = create_test_game(txn).await?;
            let user_id = create_test_user(txn, "Alice", Some("AliceUser")).await?;
            create_test_game_player(txn, game_id, user_id, 0).await?;

            // Test the adapter
            let result = players::get_display_name_by_seat(txn, game_id, 0).await?;

            assert_eq!(result, "AliceUser");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_get_display_name_by_seat_fallback_to_sub() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data with no username (should fall back to sub)
            let game_id = create_test_game(txn).await?;
            let user_id = create_test_user(txn, "bob", None).await?;
            create_test_game_player(txn, game_id, user_id, 1).await?;

            // Test the adapter
            let result = players::get_display_name_by_seat(txn, game_id, 1).await?;

            assert_eq!(result, "bob");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_get_display_name_by_seat_player_not_found() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test game but no players
            let game_id = create_test_game(txn).await?;

            // Test the adapter
            let result = players::get_display_name_by_seat(txn, game_id, 0).await;

            match result {
                Err(DomainError::NotFound(_, _)) => {
                    // Expected
                }
                _ => panic!("Expected NotFound error"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_get_display_name_by_seat_missing_user() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data with missing user (data corruption scenario)
            let game_id = create_test_game(txn).await?;

            // Create a user first, then create game_player, then delete the user
            // This simulates data corruption where the user was deleted but game_player remains
            let user_id = create_test_user(txn, "orphan", Some("OrphanUser")).await?;
            create_test_game_player(txn, game_id, user_id, 0).await?;

            // Delete the user to create the orphaned game_player
            use sea_orm::EntityTrait;
            users::Entity::delete_by_id(user_id).exec(txn).await?;

            // Test the adapter
            let result = players::get_display_name_by_seat(txn, game_id, 0).await;

            match result {
                Err(DomainError::Infra(_, msg)) => {
                    assert!(msg.contains("User not found for game player"));
                }
                Err(DomainError::NotFound(_, msg)) => {
                    assert!(msg.contains("Player not found at seat"));
                }
                Ok(display_name) => {
                    panic!("Expected error but got success: {display_name}");
                }
                Err(e) => {
                    panic!("Unexpected error type: {e:?}");
                }
            }

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
    let now = time::OffsetDateTime::now_utc();
    let game_player = game_players::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(game_id),
        user_id: Set(user_id),
        turn_order: Set(turn_order),
        is_ready: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };

    game_player.insert(txn).await?;
    Ok(())
}
