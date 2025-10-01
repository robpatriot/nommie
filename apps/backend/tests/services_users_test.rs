mod common;
mod support;

use actix_web::http::StatusCode;
use actix_web::ResponseError;
use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::entities::user_credentials;
use backend::error::AppError;
use backend::errors::ErrorCode;
use backend::infra::state::build_state;
use backend::services::users::ensure_user;
use backend::utils::unique::{unique_email, unique_str};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_ensure_user_inserts_then_reuses() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // First call - should create a new user
            let test_email = unique_email("alice");
            let test_google_sub = unique_str("google-sub");
            let user1 = ensure_user(
                test_email.clone(),
                Some("Alice".to_string()),
                test_google_sub.clone(),
                txn,
            )
            .await?;

            // Verify user was created with expected values
            assert_eq!(user1.username, Some("Alice".to_string()));
            assert!(!user1.is_ai);
            assert!(user1.id > 0); // ID should be a positive number

            // Second call with same email but different name - should return same user
            let user2 = ensure_user(
                test_email.clone(),
                Some("Alice Smith".to_string()), // Different name
                test_google_sub.clone(),         // Same google_sub
                txn,
            )
            .await?;

            // Verify idempotency - same user ID
            assert_eq!(user1.id, user2.id);
            assert_eq!(user2.username, Some("Alice".to_string())); // Username should not change

            // Verify that only one user_credentials row exists for this email
            let credential_count = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&test_email))
                .count(txn)
                .await?;

            // Should have exactly one credential row
            assert_eq!(
                credential_count, 1,
                "Should have exactly one credential row"
            );

            // Verify that the credential row has the correct user_id
            let credential = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&test_email))
                .one(txn)
                .await
                .map_err(|e| {
                    backend::error::AppError::from(backend::infra::db_errors::map_db_err(e))
                })?
                .expect("should have credential row");

            assert_eq!(
                credential.user_id, user1.id,
                "Credential should link to the correct user"
            );
            assert!(credential.last_login.is_some(), "last_login should be set");
            assert_eq!(
                credential.google_sub,
                Some(test_google_sub.clone()),
                "google_sub should be the original one set"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_ensure_user_google_sub_mismatch_policy() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let test_email = unique_email("bob");
            let original_google_sub = unique_str("google-sub-original");
            let different_google_sub = unique_str("google-sub-different");

            // Scenario 1: First login (no user/credential) → creates user + credentials, sets google_sub
            let user1 = ensure_user(
                test_email.clone(),
                Some("Bob".to_string()),
                original_google_sub.clone(),
                txn,
            )
            .await?;

            // Verify user was created with expected values
            assert_eq!(user1.username, Some("Bob".to_string()));
            assert!(!user1.is_ai);
            assert!(user1.id > 0);

            // Verify credential was created with the original google_sub
            let credential = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&test_email))
                .one(txn)
                .await
                .map_err(|e| {
                    backend::error::AppError::from(backend::infra::db_errors::map_db_err(e))
                })?
                .expect("should have credential row");

            assert_eq!(
                credential.google_sub,
                Some(original_google_sub.clone()),
                "google_sub should be set to original value"
            );

            // Scenario 2: Repeat login (same email, same google_sub) → updates last_login, succeeds
            let user2 = ensure_user(
                test_email.clone(),
                Some("Bob Smith".to_string()), // Different name
                original_google_sub.clone(),   // Same google_sub
                txn,
            )
            .await?;

            // Verify idempotency - same user ID
            assert_eq!(user1.id, user2.id);
            assert_eq!(user2.username, Some("Bob".to_string())); // Username should not change

            // Verify that only one user_credentials row exists for this email
            let credential_count = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&test_email))
                .count(txn)
                .await?;

            assert_eq!(
                credential_count, 1,
                "Should have exactly one credential row"
            );

            // Scenario 3: Repeat login (same email, credential has different google_sub) → expect 409 with GOOGLE_SUB_MISMATCH
            let error_result = ensure_user(
                test_email.clone(),
                Some("Bob".to_string()),
                different_google_sub.clone(),
                txn,
            )
            .await;

            // Verify that the original credential remains unchanged
            let credential_after_error = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&test_email))
                .one(txn)
                .await?
                .expect("should have credential row");

            assert_eq!(
                credential_after_error.google_sub,
                Some(original_google_sub.clone()),
                "google_sub should remain unchanged after mismatch error"
            );

            // Verify the error inside the transaction (status + code)
            match error_result {
                Err(err) => {
                    // HTTP status via ResponseError
                    assert_eq!(err.status_code(), StatusCode::CONFLICT);
                    // Machine-readable code via AppError helper (returns ErrorCode)
                    assert_eq!(err.code(), ErrorCode::GoogleSubMismatch);
                }
                Ok(_) => panic!("Expected Conflict error with GOOGLE_SUB_MISMATCH code"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_ensure_user_set_null_google_sub() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let test_email = unique_email("charlie");
            let google_sub = unique_str("google-sub");

            // Create a user with NULL google_sub by directly inserting into the database
            // This simulates a legacy user who doesn't have a google_sub set yet
            use backend::entities::users;
            use sea_orm::{ActiveModelTrait, NotSet, Set};

            let now = time::OffsetDateTime::now_utc();
            let user_active = users::ActiveModel {
                id: NotSet,
                sub: Set(google_sub.clone()),
                username: Set(Some("Charlie".to_string())),
                is_ai: Set(false),
                created_at: Set(now),
                updated_at: Set(now),
            };

            let user = user_active.insert(txn).await.map_err(|e| {
                backend::error::AppError::from(backend::infra::db_errors::map_db_err(e))
            })?;

            // Create credential with NULL google_sub
            let credential_active = user_credentials::ActiveModel {
                id: NotSet,
                user_id: Set(user.id),
                password_hash: Set(None),
                email: Set(test_email.clone()),
                google_sub: Set(None), // NULL google_sub
                last_login: Set(Some(now)),
                created_at: Set(now),
                updated_at: Set(now),
            };

            credential_active.insert(txn).await.map_err(|e| {
                backend::error::AppError::from(backend::infra::db_errors::map_db_err(e))
            })?;

            // Scenario 4: Repeat login (email exists, google_sub NULL) → sets google_sub to incoming, succeeds
            let updated_user = ensure_user(
                test_email.clone(),
                Some("Charlie Brown".to_string()), // Different name
                google_sub.clone(),
                txn,
            )
            .await?;

            // Verify same user ID
            assert_eq!(user.id, updated_user.id);
            assert_eq!(updated_user.username, Some("Charlie".to_string())); // Username should not change

            // Verify that the google_sub was set
            let credential_after_update = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&test_email))
                .one(txn)
                .await?
                .expect("should have credential row");

            assert_eq!(
                credential_after_update.google_sub,
                Some(google_sub.clone()),
                "google_sub should be set to incoming value"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
