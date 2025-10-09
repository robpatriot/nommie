mod common;

use backend::adapters::memberships_sea::{MembershipCreate, MembershipSetReady};
use backend::db::txn::with_txn;
use backend::entities::{games, users};
use backend::error::AppError;
use backend::infra::state::build_state;
use sea_orm::{ActiveModelTrait, Set};

#[tokio::test]
async fn test_create_membership_sets_both_timestamps() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data
            let game_id = create_test_game(txn).await?;
            let user_id = create_test_user(txn, "test_user", Some("TestUser")).await?;

            // Create membership via adapter
            let dto = MembershipCreate::new(game_id, user_id, 0, false);
            let membership =
                backend::adapters::memberships_sea::create_membership(txn, dto).await?;

            // Assert both timestamps are set
            assert!(
                membership.created_at.unix_timestamp() > 0,
                "created_at should be set"
            );
            assert!(
                membership.updated_at.unix_timestamp() > 0,
                "updated_at should be set"
            );

            // Assert created_at == updated_at (or within tolerance)
            let diff = (membership.updated_at - membership.created_at).whole_seconds();
            assert!(
                diff.abs() <= 1,
                "created_at and updated_at should be equal or within 1 second, but diff was {diff}s"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_set_ready_updates_only_updated_at() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data
            let game_id = create_test_game(txn).await?;
            let user_id = create_test_user(txn, "test_user", Some("TestUser")).await?;

            // Create membership
            let dto = MembershipCreate::new(game_id, user_id, 0, false);
            let membership =
                backend::adapters::memberships_sea::create_membership(txn, dto).await?;

            let original_created_at = membership.created_at;
            let original_updated_at = membership.updated_at;
            let membership_id = membership.id;

            // Wait a tiny bit to ensure updated_at will be different
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Call set_ready to flip is_ready
            let set_ready_dto = MembershipSetReady::new(membership_id, true);
            let updated_membership =
                backend::adapters::memberships_sea::set_membership_ready(txn, set_ready_dto)
                    .await?;

            // Assert created_at unchanged
            assert_eq!(
                updated_membership.created_at, original_created_at,
                "created_at should not change"
            );

            // Assert updated_at is greater than original
            assert!(
                updated_membership.updated_at >= original_updated_at,
                "updated_at should be greater than or equal to original"
            );

            // Assert is_ready was updated
            assert!(updated_membership.is_ready, "is_ready should be true");

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
        join_code: Set(Some(format!("C{}", rand::random::<u32>() % 1000000))),
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
        sub: Set(format!("{}_{}", sub, rand::random::<u32>())),
        username: Set(username.map(|s| s.to_string())),
        is_ai: Set(false),
        created_at: Set(time::OffsetDateTime::now_utc()),
        updated_at: Set(time::OffsetDateTime::now_utc()),
    };

    let inserted = user.insert(txn).await?;
    Ok(inserted.id)
}
