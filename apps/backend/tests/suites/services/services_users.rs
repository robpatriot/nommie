use actix_web::http::StatusCode;
use actix_web::ResponseError;
use backend::db::txn::with_txn;
use backend::entities::user_credentials;
use backend::services::users::UserService;
use backend::{AppError, ErrorCode};
use backend_test_support::unique_helpers::{unique_email, unique_str};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

use crate::support::build_test_state;

#[tokio::test]
async fn test_ensure_user_inserts_then_reuses() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // First call - should create a new user
            let test_email = unique_email("alice");
            let test_google_sub = unique_str("google-sub");
            let service = UserService;
            let user1 = service
                .ensure_user(txn, &test_email, Some("Alice"), &test_google_sub, None)
                .await?;

            // Verify user was created with expected values
            assert_eq!(user1.username, Some("Alice".to_string()));
            assert!(!user1.is_ai);
            assert!(user1.id > 0); // ID should be a positive number

            // Second call with same email but different name - should return same user
            let user2 = service
                .ensure_user(
                    txn,
                    &test_email,
                    Some("Alice Smith"), // Different name
                    &test_google_sub,    // Same google_sub
                    None,
                )
                .await?;

            // Verify idempotency - same user ID
            assert_eq!(user1.id, user2.id);
            assert_eq!(user2.username, Some("Alice".to_string())); // Username should not change

            // Verify that only one user_credentials row exists for this email
            // Note: email is normalized (lowercased) before storage
            let normalized_email = test_email.to_lowercase();
            let credential_count = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&normalized_email))
                .count(txn)
                .await?;

            // Should have exactly one credential row
            assert_eq!(
                credential_count, 1,
                "Should have exactly one credential row"
            );

            // Verify that the credential row has the correct user_id
            let credential = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&normalized_email))
                .one(txn)
                .await
                .map_err(|e| backend::AppError::from(backend::infra::db_errors::map_db_err(e)))?
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
async fn test_ensure_user_google_sub_mismatch_policy() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let test_email = unique_email("bob");
            let original_google_sub = unique_str("google-sub-original");
            let different_google_sub = unique_str("google-sub-different");
            let service = UserService;

            // Scenario 1: First login (no user/credential) → creates user + credentials, sets google_sub
            let user1 = service
                .ensure_user(txn, &test_email, Some("Bob"), &original_google_sub, None)
                .await?;

            // Verify user was created with expected values
            assert_eq!(user1.username, Some("Bob".to_string()));
            assert!(!user1.is_ai);
            assert!(user1.id > 0);

            // Verify credential was created with the original google_sub
            // Note: email is normalized (lowercased) before storage
            let normalized_email = test_email.to_lowercase();
            let credential = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&normalized_email))
                .one(txn)
                .await
                .map_err(|e| backend::AppError::from(backend::infra::db_errors::map_db_err(e)))?
                .expect("should have credential row");

            assert_eq!(
                credential.google_sub,
                Some(original_google_sub.clone()),
                "google_sub should be set to original value"
            );

            // Scenario 2: Repeat login (same email, same google_sub) → updates last_login, succeeds
            let user2 = service
                .ensure_user(
                    txn,
                    &test_email,
                    Some("Bob Smith"),    // Different name
                    &original_google_sub, // Same google_sub
                    None,                 // No allowlist for tests
                )
                .await?;

            // Verify idempotency - same user ID
            assert_eq!(user1.id, user2.id);
            assert_eq!(user2.username, Some("Bob".to_string())); // Username should not change

            // Verify that only one user_credentials row exists for this email
            let credential_count = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&normalized_email))
                .count(txn)
                .await?;

            assert_eq!(
                credential_count, 1,
                "Should have exactly one credential row"
            );

            // Scenario 3: Repeat login (same email, credential has different google_sub) → expect 409 with GOOGLE_SUB_MISMATCH
            let error_result = service
                .ensure_user(txn, &test_email, Some("Bob"), &different_google_sub, None)
                .await;

            // Verify that the original credential remains unchanged
            let credential_after_error = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&normalized_email))
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
                Err(domain_err) => {
                    let err: AppError = domain_err.into();
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
async fn test_ensure_user_set_null_google_sub() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let test_email = unique_email("charlie");
            let google_sub = unique_str("google-sub");
            let service = UserService;

            // Create a user with NULL google_sub by directly inserting into the database
            // This simulates a user who doesn't have a google_sub set yet
            use backend::entities::users;
            use sea_orm::{ActiveModelTrait, NotSet, Set};

            // Use a unique sub for this test to avoid constraint violations
            let user_sub = unique_str("user-sub");
            let normalized_email = test_email.to_lowercase();

            let now = time::OffsetDateTime::now_utc();
            let user_active = users::ActiveModel {
                id: NotSet,
                sub: Set(user_sub.clone()),
                username: Set(Some("Charlie".to_string())),
                is_ai: Set(false),
                created_at: Set(now),
                updated_at: Set(now),
            };

            let user = user_active
                .insert(txn)
                .await
                .map_err(|e| backend::AppError::from(backend::infra::db_errors::map_db_err(e)))?;

            // Create credential with NULL google_sub
            // Note: store normalized email to match service behavior
            let credential_active = user_credentials::ActiveModel {
                id: NotSet,
                user_id: Set(user.id),
                password_hash: Set(None),
                email: Set(normalized_email.clone()),
                google_sub: Set(None), // NULL google_sub
                last_login: Set(Some(now)),
                created_at: Set(now),
                updated_at: Set(now),
            };

            credential_active
                .insert(txn)
                .await
                .map_err(|e| backend::AppError::from(backend::infra::db_errors::map_db_err(e)))?;

            // Scenario 4: Repeat login (email exists, google_sub NULL) → sets google_sub to incoming, succeeds
            let updated_user = service
                .ensure_user(
                    txn,
                    &test_email,
                    Some("Charlie Brown"), // Different name
                    &google_sub,
                    None,
                )
                .await?;

            // Verify same user ID
            assert_eq!(user.id, updated_user.id);
            assert_eq!(updated_user.username, Some("Charlie".to_string())); // Username should not change

            // Verify that the google_sub was set
            let credential_after_update = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&normalized_email))
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

#[tokio::test]
async fn test_email_normalization_case_and_whitespace() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            // Create user with uppercase email and surrounding whitespace
            let user1 = service
                .ensure_user(
                    txn,
                    "  ALICE@EXAMPLE.COM  ",
                    Some("Alice"),
                    &google_sub,
                    None,
                )
                .await?;

            // Verify user was created
            assert_eq!(user1.username, Some("Alice".to_string()));

            // Try to log in with lowercase email (should find the same user)
            let user2 = service
                .ensure_user(txn, "alice@example.com", Some("Alice"), &google_sub, None)
                .await?;

            // Should be the same user
            assert_eq!(user1.id, user2.id);

            // Try with mixed case
            let user3 = service
                .ensure_user(txn, "Alice@Example.Com", Some("Alice"), &google_sub, None)
                .await?;

            // Should still be the same user
            assert_eq!(user1.id, user3.id);

            // Verify that only one credential row exists
            let credential_count = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq("alice@example.com"))
                .count(txn)
                .await?;

            assert_eq!(
                credential_count, 1,
                "Should have exactly one credential row"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_email_normalization_unicode_nfkc() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            // Create user with composed Unicode (é as single character U+00E9)
            let email_composed = "café@example.com";
            let user1 = service
                .ensure_user(txn, email_composed, Some("User"), &google_sub, None)
                .await?;

            // Try to log in with decomposed Unicode (é as e + combining acute accent U+0065 U+0301)
            // This should normalize to the same value via NFKC
            let email_decomposed = "cafe\u{0301}@example.com";
            let user2 = service
                .ensure_user(txn, email_decomposed, Some("User"), &google_sub, None)
                .await?;

            // Should be the same user (NFKC normalization ensures this)
            assert_eq!(
                user1.id, user2.id,
                "Unicode variants should normalize to the same user"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_email_validation_missing_at_symbol() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            // Try to create user with email missing @ symbol
            let result = service
                .ensure_user(txn, "invalidemail.com", Some("User"), &google_sub, None)
                .await;

            // Should fail with InvalidEmail error
            match result {
                Err(domain_err) => {
                    let err: AppError = domain_err.into();
                    assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
                    assert_eq!(err.code(), ErrorCode::InvalidEmail);
                }
                Ok(_) => panic!("Expected InvalidEmail error"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_email_validation_multiple_at_symbols() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            // Try to create user with email having multiple @ symbols
            let result = service
                .ensure_user(txn, "user@@example.com", Some("User"), &google_sub, None)
                .await;

            // Should fail with InvalidEmail error
            match result {
                Err(domain_err) => {
                    let err: AppError = domain_err.into();
                    assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
                    assert_eq!(err.code(), ErrorCode::InvalidEmail);
                }
                Ok(_) => panic!("Expected InvalidEmail error"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_email_validation_empty_local_part() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            // Try to create user with email having empty local part
            let result = service
                .ensure_user(txn, "@example.com", Some("User"), &google_sub, None)
                .await;

            // Should fail with InvalidEmail error
            match result {
                Err(domain_err) => {
                    let err: AppError = domain_err.into();
                    assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
                    assert_eq!(err.code(), ErrorCode::InvalidEmail);
                }
                Ok(_) => panic!("Expected InvalidEmail error"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_email_validation_empty_domain() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            // Try to create user with email having empty domain part
            let result = service
                .ensure_user(txn, "user@", Some("User"), &google_sub, None)
                .await;

            // Should fail with InvalidEmail error
            match result {
                Err(domain_err) => {
                    let err: AppError = domain_err.into();
                    assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
                    assert_eq!(err.code(), ErrorCode::InvalidEmail);
                }
                Ok(_) => panic!("Expected InvalidEmail error"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_email_validation_whitespace_only() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            // Try to create user with email that becomes empty after trimming
            let result = service
                .ensure_user(txn, "   ", Some("User"), &google_sub, None)
                .await;

            // Should fail with InvalidEmail error
            match result {
                Err(domain_err) => {
                    let err: AppError = domain_err.into();
                    assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
                    assert_eq!(err.code(), ErrorCode::InvalidEmail);
                }
                Ok(_) => panic!("Expected InvalidEmail error"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
