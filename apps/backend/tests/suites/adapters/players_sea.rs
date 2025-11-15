use backend::db::txn::with_txn;
use backend::entities::users;
use backend::error::AppError;
use backend::errors::domain::DomainError;
use backend::repos::{ai_profiles, players};
use rand::random;
use sea_orm::{ActiveModelTrait, NotSet, Set};
use time::OffsetDateTime;

use crate::support::build_test_state;
use crate::support::db_memberships::create_test_game_player;
use crate::support::factory::{create_test_game, create_test_user};

#[tokio::test]
async fn test_get_display_name_by_seat_success() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data
            let game_id = create_test_game(txn).await?;
            let user_id = create_test_user(txn, "Alice", Some("AliceUser")).await?;
            let _ = create_test_game_player(txn, game_id, user_id, 0).await?;

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
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data with no username (should fall back to sub)
            let game_id = create_test_game(txn).await?;
            let user_id = create_test_user(txn, "bob", None).await?;
            let _ = create_test_game_player(txn, game_id, user_id, 1).await?;

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
async fn test_get_display_name_by_seat_ai_profile() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let ai_profile = ai_profiles::find_by_registry_variant(
                txn,
                backend::ai::RandomPlayer::NAME,
                backend::ai::RandomPlayer::VERSION,
                "default",
            )
            .await?
            .expect("catalog profile missing");

            let game_id = create_test_game(txn).await?;
            let _ = crate::support::db_memberships::create_test_ai_game_player(
                txn,
                game_id,
                ai_profile.id,
                2,
                false,
            )
            .await?;

            let result = players::get_display_name_by_seat(txn, game_id, 2).await?;
            assert_eq!(result, ai_profile.display_name);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_get_display_name_by_seat_player_not_found() -> Result<(), AppError> {
    let state = build_test_state().await?;

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
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data with missing user (data corruption scenario)
            let game_id = create_test_game(txn).await?;

            // Create a user first, then create game_player, then delete the user
            // This simulates data corruption where the user was deleted but game_player remains
            let user_id = create_test_user(txn, "orphan", Some("OrphanUser")).await?;
            let _ = create_test_game_player(txn, game_id, user_id, 0).await?;

            // Delete the user to create the orphaned game_player
            use backend::entities::users;
            use sea_orm::EntityTrait;
            users::Entity::delete_by_id(user_id).exec(txn).await?;

            // Test the adapter
            let result = players::get_display_name_by_seat(txn, game_id, 0).await;

            match result {
                Err(DomainError::Infra(kind, _)) => {
                    use backend::errors::domain::InfraErrorKind;
                    assert_eq!(kind, InfraErrorKind::DataCorruption);
                }
                Err(DomainError::NotFound(kind, _)) => {
                    use backend::errors::domain::NotFoundKind;
                    assert_eq!(kind, NotFoundKind::Player);
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
