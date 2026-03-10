use actix_web::http::StatusCode;
use actix_web::ResponseError;
use backend::auth::google::VerifiedGoogleClaims;
use backend::db::require_db;
use backend::db::txn::with_txn;
use backend::entities::user_auth_identities;
use backend::entities::users::UserRole;
use backend::services::users::UserService;
use backend::{AppError, ErrorCode};
use backend_test_support::unique_helpers::{unique_email, unique_str};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

use crate::support::auth::seed_admission_email;
use crate::support::build_test_state;

const PROVIDER_GOOGLE: &str = "google";

fn claims(email: &str, name: Option<&str>, sub: &str) -> VerifiedGoogleClaims {
    VerifiedGoogleClaims {
        sub: sub.to_string(),
        email: email.to_string(),
        name: name.map(String::from),
    }
}

#[tokio::test]
async fn test_ensure_user_inserts_then_reuses() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required");
    seed_admission_email(&db, "*@example.test", false).await;

    let admission_mode = state.config.admission_mode;
    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // First call - should create a new user
            let test_email = unique_email("alice");
            let test_google_sub = unique_str("google-sub");
            let service = UserService;
            let user1 = service
                .ensure_user(
                    txn,
                    &claims(&test_email, Some("Alice"), &test_google_sub),
                    admission_mode,
                )
                .await?;

            // Verify user was created with expected values
            assert_eq!(user1.username, Some("Alice".to_string()));
            assert!(!user1.is_ai);
            assert!(user1.id > 0); // ID should be a positive number

            // Second call with same email but different name - should return same user
            let user2 = service
                .ensure_user(
                    txn,
                    &claims(&test_email, Some("Alice Smith"), &test_google_sub),
                    admission_mode,
                )
                .await?;

            // Verify idempotency - same user ID
            assert_eq!(user1.id, user2.id);
            assert_eq!(user2.username, Some("Alice".to_string())); // Username should not change

            // Verify that only one identity row exists for this email (provider=google)
            let normalized_email = test_email.to_lowercase();
            let identity_count = user_auth_identities::Entity::find()
                .filter(user_auth_identities::Column::Provider.eq(PROVIDER_GOOGLE))
                .filter(user_auth_identities::Column::Email.eq(&normalized_email))
                .count(txn)
                .await?;

            assert_eq!(identity_count, 1, "Should have exactly one identity row");

            // Verify that the identity row has the correct user_id and provider_user_id
            let identity = user_auth_identities::Entity::find()
                .filter(user_auth_identities::Column::Provider.eq(PROVIDER_GOOGLE))
                .filter(user_auth_identities::Column::Email.eq(&normalized_email))
                .one(txn)
                .await
                .map_err(|e| backend::AppError::from(backend::infra::db_errors::map_db_err(e)))?
                .expect("should have identity row");

            assert_eq!(
                identity.user_id, user1.id,
                "Identity should link to the correct user"
            );
            assert!(
                identity.last_login_at.is_some(),
                "last_login_at should be set"
            );
            assert_eq!(
                identity.provider_user_id, test_google_sub,
                "provider_user_id should be the original one set"
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
    let db = require_db(&state).expect("DB required");
    seed_admission_email(&db, "*@example.test", false).await;

    let admission_mode = state.config.admission_mode;
    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let test_email = unique_email("bob");
            let original_google_sub = unique_str("google-sub-original");
            let different_google_sub = unique_str("google-sub-different");
            let service = UserService;

            // Scenario 1: First login (no user/identity) → creates user + identity
            let user1 = service
                .ensure_user(
                    txn,
                    &claims(&test_email, Some("Bob"), &original_google_sub),
                    admission_mode,
                )
                .await?;

            assert_eq!(user1.username, Some("Bob".to_string()));
            assert!(!user1.is_ai);
            assert!(user1.id > 0);

            let normalized_email = test_email.to_lowercase();
            let identity = user_auth_identities::Entity::find()
                .filter(user_auth_identities::Column::Provider.eq(PROVIDER_GOOGLE))
                .filter(user_auth_identities::Column::Email.eq(&normalized_email))
                .one(txn)
                .await
                .map_err(|e| backend::AppError::from(backend::infra::db_errors::map_db_err(e)))?
                .expect("should have identity row");

            assert_eq!(
                identity.provider_user_id, original_google_sub,
                "provider_user_id should be set to original value"
            );

            // Scenario 2: Repeat login (same email, same google_sub) → updates last_login, succeeds
            let user2 = service
                .ensure_user(
                    txn,
                    &claims(&test_email, Some("Bob Smith"), &original_google_sub),
                    admission_mode,
                )
                .await?;

            assert_eq!(user1.id, user2.id);
            assert_eq!(user2.username, Some("Bob".to_string()));

            let identity_count = user_auth_identities::Entity::find()
                .filter(user_auth_identities::Column::Provider.eq(PROVIDER_GOOGLE))
                .filter(user_auth_identities::Column::Email.eq(&normalized_email))
                .count(txn)
                .await?;

            assert_eq!(identity_count, 1, "Should have exactly one identity row");

            // Scenario 3: Repeat login (same email, different google_sub) → expect 409 GOOGLE_SUB_MISMATCH
            let error_result = service
                .ensure_user(
                    txn,
                    &claims(&test_email, Some("Bob"), &different_google_sub),
                    admission_mode,
                )
                .await;

            let identity_after_error = user_auth_identities::Entity::find()
                .filter(user_auth_identities::Column::Provider.eq(PROVIDER_GOOGLE))
                .filter(user_auth_identities::Column::Email.eq(&normalized_email))
                .one(txn)
                .await?
                .expect("should have identity row");

            assert_eq!(
                identity_after_error.provider_user_id, original_google_sub,
                "provider_user_id should remain unchanged after mismatch error"
            );

            match error_result {
                Err(domain_err) => {
                    let err: AppError = domain_err.into();
                    assert_eq!(err.status_code(), StatusCode::CONFLICT);
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
async fn test_email_normalization_case_and_whitespace() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required");
    seed_admission_email(&db, "alice@example.com", false).await;

    let admission_mode = state.config.admission_mode;
    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            let user1 = service
                .ensure_user(
                    txn,
                    &claims("  ALICE@EXAMPLE.COM  ", Some("Alice"), &google_sub),
                    admission_mode,
                )
                .await?;

            assert_eq!(user1.username, Some("Alice".to_string()));

            let user2 = service
                .ensure_user(
                    txn,
                    &claims("alice@example.com", Some("Alice"), &google_sub),
                    admission_mode,
                )
                .await?;

            assert_eq!(user1.id, user2.id);

            let user3 = service
                .ensure_user(
                    txn,
                    &claims("Alice@Example.Com", Some("Alice"), &google_sub),
                    admission_mode,
                )
                .await?;

            assert_eq!(user1.id, user3.id);

            let identity_count = user_auth_identities::Entity::find()
                .filter(user_auth_identities::Column::Provider.eq(PROVIDER_GOOGLE))
                .filter(user_auth_identities::Column::Email.eq("alice@example.com"))
                .count(txn)
                .await?;

            assert_eq!(identity_count, 1, "Should have exactly one identity row");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_email_normalization_unicode_nfkc() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required");
    seed_admission_email(&db, "café@example.com", false).await;

    let admission_mode = state.config.admission_mode;
    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            let email_composed = "café@example.com";
            let user1 = service
                .ensure_user(
                    txn,
                    &claims(email_composed, Some("User"), &google_sub),
                    admission_mode,
                )
                .await?;

            let email_decomposed = "cafe\u{0301}@example.com";
            let user2 = service
                .ensure_user(
                    txn,
                    &claims(email_decomposed, Some("User"), &google_sub),
                    admission_mode,
                )
                .await?;

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
    let admission_mode = state.config.admission_mode;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            let result = service
                .ensure_user(
                    txn,
                    &claims("invalidemail.com", Some("User"), &google_sub),
                    admission_mode,
                )
                .await;

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
    let admission_mode = state.config.admission_mode;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            let result = service
                .ensure_user(
                    txn,
                    &claims("user@@example.com", Some("User"), &google_sub),
                    admission_mode,
                )
                .await;

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
    let admission_mode = state.config.admission_mode;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            let result = service
                .ensure_user(
                    txn,
                    &claims("@example.com", Some("User"), &google_sub),
                    admission_mode,
                )
                .await;

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
    let admission_mode = state.config.admission_mode;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            let result = service
                .ensure_user(
                    txn,
                    &claims("user@", Some("User"), &google_sub),
                    admission_mode,
                )
                .await;

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
    let admission_mode = state.config.admission_mode;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let google_sub = unique_str("google-sub");

            let result = service
                .ensure_user(
                    txn,
                    &claims("   ", Some("User"), &google_sub),
                    admission_mode,
                )
                .await;

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
async fn test_first_login_role_normal_allowed_creates_user() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required");
    seed_admission_email(&db, "alice@example.com", false).await;

    let admission_mode = state.config.admission_mode;
    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let user = service
                .ensure_user(
                    txn,
                    &claims(
                        "alice@example.com",
                        Some("Alice"),
                        &unique_str("google-sub"),
                    ),
                    admission_mode,
                )
                .await?;
            assert_eq!(user.role, UserRole::User);
            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_first_login_role_exact_admin_creates_admin() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required");
    seed_admission_email(&db, "admin@example.com", true).await;

    let admission_mode = state.config.admission_mode;
    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let user = service
                .ensure_user(
                    txn,
                    &claims(
                        "admin@example.com",
                        Some("Admin"),
                        &unique_str("google-sub"),
                    ),
                    admission_mode,
                )
                .await?;
            assert_eq!(user.role, UserRole::Admin);
            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_first_login_role_wildcard_only_creates_user() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required");
    seed_admission_email(&db, "*@example.test", false).await;

    let admission_mode = state.config.admission_mode;
    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let service = UserService;
            let user = service
                .ensure_user(
                    txn,
                    &claims(
                        &unique_email("wildcard-user"),
                        Some("User"),
                        &unique_str("google-sub"),
                    ),
                    admission_mode,
                )
                .await?;
            assert_eq!(
                user.role,
                UserRole::User,
                "wildcard-only must yield role=user"
            );
            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_downgraded_user_repeat_login_does_not_reevaluate_role() -> Result<(), AppError> {
    use backend::db::txn::SharedTxn;
    use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};

    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required");
    let email = "downgraded@example.com";
    seed_admission_email(&db, email, true).await;

    let google_sub = unique_str("google-sub");
    let admission_mode = state.config.admission_mode;

    let shared = SharedTxn::open(&db).await?;
    let txn = shared.transaction();

    let service = UserService;
    let user = service
        .ensure_user(
            txn,
            &claims(email, Some("Admin"), &google_sub),
            admission_mode,
        )
        .await?;
    assert_eq!(user.role, UserRole::Admin);

    let identity = user_auth_identities::Entity::find()
        .filter(user_auth_identities::Column::Provider.eq(PROVIDER_GOOGLE))
        .filter(user_auth_identities::Column::Email.eq(email))
        .one(txn)
        .await
        .map_err(|e| AppError::from(backend::infra::db_errors::map_db_err(e)))?
        .expect("identity should exist after ensure_user");

    let user_model = backend::entities::users::Entity::find_by_id(identity.user_id)
        .one(txn)
        .await
        .map_err(|e| AppError::from(backend::infra::db_errors::map_db_err(e)))?
        .expect("user should exist");

    let mut active: backend::entities::users::ActiveModel = user_model.into();
    active.role = ActiveValue::Set(UserRole::User);
    active
        .update(txn)
        .await
        .map_err(|e| AppError::from(backend::infra::db_errors::map_db_err(e)))?;

    let user_after = service
        .ensure_user(
            txn,
            &claims(email, Some("Admin"), &google_sub),
            admission_mode,
        )
        .await?;
    assert_eq!(
        user_after.role,
        UserRole::User,
        "repeat login must not re-evaluate role; role stays user after manual downgrade"
    );

    shared.rollback().await?;
    Ok(())
}
