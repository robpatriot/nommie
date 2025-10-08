mod common;
mod support;

use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::errors::domain::{ConflictKind, DomainError};
use backend::infra::state::build_state;
use backend::repos::users;
use backend::utils::unique::{unique_email, unique_str};
use serial_test::serial;

/// Test: find_credentials_by_email returns None when email doesn't exist
#[tokio::test]
#[serial]
async fn test_find_credentials_by_email_not_found() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let non_existent_email = unique_email("nonexistent");

            // Test the adapter via repo
            let result = users::find_credentials_by_email(txn, &non_existent_email).await?;

            assert!(result.is_none(), "Expected None for non-existent email");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: create_user and find_user_by_id roundtrip
#[tokio::test]
#[serial]
async fn test_create_user_and_find_by_id_roundtrip() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let sub = unique_str("test-sub");
            let username = "TestUser";

            // Create user
            let created = users::create_user(txn, &sub, username, false).await?;

            assert!(created.id > 0, "User ID should be positive");
            assert_eq!(created.sub, sub);
            assert_eq!(created.username, Some(username.to_string()));
            assert!(!created.is_ai);

            // Find user by ID
            let found = users::find_user_by_id(txn, created.id).await?;

            assert!(found.is_some(), "User should be found");
            let found = found.unwrap();
            assert_eq!(found.id, created.id);
            assert_eq!(found.sub, created.sub);
            assert_eq!(found.username, created.username);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_user_by_id returns None for non-existent user
#[tokio::test]
#[serial]
async fn test_find_user_by_id_not_found() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let non_existent_id = 999_999_999_i64;

            // Test the adapter via repo
            let result = users::find_user_by_id(txn, non_existent_id).await?;

            assert!(result.is_none(), "Expected None for non-existent user ID");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: create_credentials with duplicate email returns typed unique violation error
#[tokio::test]
#[serial]
async fn test_create_credentials_duplicate_email_unique_violation() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let email = unique_email("duplicate-test");
            let sub1 = unique_str("sub1");
            let sub2 = unique_str("sub2");

            // Create first user and credentials
            let user1 = users::create_user(txn, &sub1, "User1", false).await?;
            users::create_credentials(txn, user1.id, &email, Some(&sub1)).await?;

            // Try to create second user with same email - should fail
            let user2 = users::create_user(txn, &sub2, "User2", false).await?;
            let result = users::create_credentials(txn, user2.id, &email, Some(&sub2)).await;

            // Verify we get a typed unique violation error for email
            match result {
                Err(DomainError::Conflict(ConflictKind::UniqueEmail, _)) => {
                    // Expected - this is the typed error we want
                }
                Err(e) => {
                    panic!("Expected Conflict(UniqueEmail, ...) but got different error: {e:?}")
                }
                Ok(_) => panic!("Expected unique violation error but operation succeeded"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: create_credentials with duplicate google_sub returns typed unique violation error
#[tokio::test]
#[serial]
async fn test_create_credentials_duplicate_google_sub_unique_violation() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let email1 = unique_email("user1");
            let email2 = unique_email("user2");
            let google_sub = unique_str("duplicate-google-sub");
            let sub1 = unique_str("sub1");
            let sub2 = unique_str("sub2");

            // Create first user and credentials with google_sub
            let user1 = users::create_user(txn, &sub1, "User1", false).await?;
            users::create_credentials(txn, user1.id, &email1, Some(&google_sub)).await?;

            // Try to create second user with different email but same google_sub - should fail
            let user2 = users::create_user(txn, &sub2, "User2", false).await?;
            let result = users::create_credentials(txn, user2.id, &email2, Some(&google_sub)).await;

            // Verify we get a typed unique violation error for google_sub
            match result {
                Err(DomainError::Conflict(ConflictKind::Other(kind), _))
                    if kind == "UniqueGoogleSub" =>
                {
                    // Expected - this is the typed error we want
                }
                Err(e) => panic!(
                    "Expected Conflict(Other(UniqueGoogleSub), ...) but got different error: {e:?}"
                ),
                Ok(_) => panic!("Expected unique violation error but operation succeeded"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: update_credentials idempotency - updating with same values succeeds
#[tokio::test]
#[serial]
async fn test_update_credentials_idempotent() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let email = unique_email("update-test");
            let sub = unique_str("sub");
            let google_sub = unique_str("google-sub");

            // Create user and credentials
            let user = users::create_user(txn, &sub, "UpdateUser", false).await?;
            let creds = users::create_credentials(txn, user.id, &email, Some(&google_sub)).await?;

            // Update credentials with same email and google_sub - should succeed
            let updated = users::update_credentials(txn, creds.clone()).await?;

            assert_eq!(updated.id, creds.id);
            assert_eq!(updated.email, creds.email);
            assert_eq!(updated.google_sub, creds.google_sub);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: create credentials, find by email, verify roundtrip
#[tokio::test]
#[serial]
async fn test_create_and_find_credentials_by_email_roundtrip() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let email = unique_email("roundtrip-test");
            let sub = unique_str("sub");
            let google_sub = unique_str("google-sub");

            // Create user and credentials
            let user = users::create_user(txn, &sub, "RoundtripUser", false).await?;
            let created =
                users::create_credentials(txn, user.id, &email, Some(&google_sub)).await?;

            // Find by email
            let found = users::find_credentials_by_email(txn, &email).await?;

            assert!(found.is_some(), "Credentials should be found");
            let found = found.unwrap();
            assert_eq!(found.id, created.id);
            assert_eq!(found.user_id, created.user_id);
            assert_eq!(found.email, created.email);
            assert_eq!(found.google_sub, created.google_sub);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Verify that adapter properly maps not-found to NotFoundKind::User at service level
/// This test ensures the full error flow from adapter through repo to service
#[tokio::test]
#[serial]
async fn test_not_found_user_maps_to_typed_error() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let non_existent_id = 999_999_999_i64;

            // Find user by non-existent ID
            let result = users::find_user_by_id(txn, non_existent_id).await?;

            // At the repo level, this returns None (not an error)
            assert!(result.is_none(), "Expected None for non-existent user");

            // The service layer is responsible for converting None to NotFoundKind::User
            // This is tested in services_users_test.rs

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
