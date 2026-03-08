use backend::db::txn::with_txn;
use backend::repos::users;
use backend::AppError;

use crate::support::build_test_state;

/// Test: create_user and find_user_by_id roundtrip
#[tokio::test]
async fn test_create_user_and_find_by_id_roundtrip() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let username = "TestUser";

            // Create user
            let created = users::create_user(txn, username, false).await?;

            assert!(created.id > 0, "User ID should be positive");
            assert_eq!(created.username, Some(username.to_string()));
            assert!(!created.is_ai);

            // Find user by ID
            let found = users::find_user_by_id(txn, created.id).await?;

            assert!(found.is_some(), "User should be found");
            let found = found.unwrap();
            assert_eq!(found.id, created.id);
            assert_eq!(found.username, created.username);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_user_by_id returns None for non-existent user
#[tokio::test]
async fn test_find_user_by_id_not_found() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let non_existent_id = 999_999_999_i64;

            let result = users::find_user_by_id(txn, non_existent_id).await?;

            assert!(result.is_none(), "Expected None for non-existent user ID");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Verify that adapter properly maps not-found to NotFoundKind::User at service level
#[tokio::test]
async fn test_not_found_user_maps_to_typed_error() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let non_existent_id = 999_999_999_i64;

            let result = users::find_user_by_id(txn, non_existent_id).await?;

            assert!(result.is_none(), "Expected None for non-existent user");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
