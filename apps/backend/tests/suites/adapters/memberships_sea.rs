use backend::adapters::memberships_sea::{MembershipCreate, MembershipSetReady};
use backend::db::txn::with_txn;
use backend::error::AppError;

use crate::support::build_test_state;
use crate::support::factory::{create_test_game_with_options, create_test_user_with_randomization};

#[tokio::test]
async fn test_create_membership_sets_both_timestamps() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data
            let game_id = create_test_game_with_options(txn, None, Some(true)).await?;
            let user_id =
                create_test_user_with_randomization(txn, "test_user", Some("TestUser"), true)
                    .await?;

            // Create membership via adapter
            let dto = MembershipCreate::new(
                game_id,
                backend::entities::game_players::PlayerKind::Human,
                Some(user_id),
                None,
                0,
                false,
            );
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
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create test data
            let game_id = create_test_game_with_options(txn, None, Some(true)).await?;
            let user_id =
                create_test_user_with_randomization(txn, "test_user", Some("TestUser"), true)
                    .await?;

            // Create membership
            let dto = MembershipCreate::new(
                game_id,
                backend::entities::game_players::PlayerKind::Human,
                Some(user_id),
                None,
                0,
                false,
            );
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
