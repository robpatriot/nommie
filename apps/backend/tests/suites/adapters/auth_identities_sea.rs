use backend::db::txn::with_txn;
use backend::errors::domain::{ConflictKind, DomainError};
use backend::repos::{auth_identities, users};
use backend::AppError;
use backend_test_support::unique_helpers::{unique_email, unique_str};

use crate::support::build_test_state;

const PROVIDER_GOOGLE: &str = "google";

/// Test: find_by_provider_user_id returns None when identity doesn't exist
#[tokio::test]
async fn test_find_by_provider_user_id_not_found() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let result =
                auth_identities::find_by_provider_user_id(txn, PROVIDER_GOOGLE, "non-existent-id")
                    .await?;

            assert!(result.is_none(), "Expected None for non-existent identity");

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: create_identity and find_by_provider_user_id roundtrip
#[tokio::test]
async fn test_create_and_find_identity_roundtrip() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let email = unique_email("roundtrip");
            let provider_user_id = unique_str("google-sub");

            let user = users::create_user(
                txn,
                "TestUser",
                false,
                backend::entities::users::UserRole::User,
            )
            .await?;
            let created = auth_identities::create_identity(
                txn,
                user.id,
                PROVIDER_GOOGLE,
                &provider_user_id,
                &email,
            )
            .await?;

            let found =
                auth_identities::find_by_provider_user_id(txn, PROVIDER_GOOGLE, &provider_user_id)
                    .await?;

            assert!(found.is_some(), "Identity should be found");
            let found = found.unwrap();
            assert_eq!(found.id, created.id);
            assert_eq!(found.user_id, created.user_id);
            assert_eq!(found.provider_user_id, created.provider_user_id);
            assert_eq!(found.email, created.email);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: create_identity with duplicate (provider, provider_user_id) returns unique violation
#[tokio::test]
async fn test_create_identity_duplicate_provider_user_id() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let email1 = unique_email("user1");
            let email2 = unique_email("user2");
            let provider_user_id = unique_str("duplicate-google-sub");

            let user1 = users::create_user(
                txn,
                "User1",
                false,
                backend::entities::users::UserRole::User,
            )
            .await?;
            auth_identities::create_identity(
                txn,
                user1.id,
                PROVIDER_GOOGLE,
                &provider_user_id,
                &email1,
            )
            .await?;

            let user2 = users::create_user(
                txn,
                "User2",
                false,
                backend::entities::users::UserRole::User,
            )
            .await?;
            let result = auth_identities::create_identity(
                txn,
                user2.id,
                PROVIDER_GOOGLE,
                &provider_user_id,
                &email2,
            )
            .await;

            match result {
                Err(DomainError::Conflict(ConflictKind::Other(kind), _))
                    if kind == "UniqueGoogleSub" => {}
                Err(e) => panic!("Expected Conflict(Other(UniqueGoogleSub), ...) but got: {e:?}"),
                Ok(_) => panic!("Expected unique violation error but operation succeeded"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: create_identity with duplicate (provider, email) returns unique violation
#[tokio::test]
async fn test_create_identity_duplicate_provider_email() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let email = unique_email("duplicate");
            let sub1 = unique_str("sub1");
            let sub2 = unique_str("sub2");

            let user1 = users::create_user(
                txn,
                "User1",
                false,
                backend::entities::users::UserRole::User,
            )
            .await?;
            auth_identities::create_identity(txn, user1.id, PROVIDER_GOOGLE, &sub1, &email).await?;

            let user2 = users::create_user(
                txn,
                "User2",
                false,
                backend::entities::users::UserRole::User,
            )
            .await?;
            let result =
                auth_identities::create_identity(txn, user2.id, PROVIDER_GOOGLE, &sub2, &email)
                    .await;

            match result {
                Err(DomainError::Conflict(ConflictKind::UniqueEmail, _)) => {}
                Err(e) => panic!("Expected Conflict(UniqueEmail, ...) but got: {e:?}"),
                Ok(_) => panic!("Expected unique violation error but operation succeeded"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: update_identity idempotency
#[tokio::test]
async fn test_update_identity_idempotent() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let email = unique_email("update");
            let provider_user_id = unique_str("google-sub");

            let user = users::create_user(
                txn,
                "UpdateUser",
                false,
                backend::entities::users::UserRole::User,
            )
            .await?;
            let identity = auth_identities::create_identity(
                txn,
                user.id,
                PROVIDER_GOOGLE,
                &provider_user_id,
                &email,
            )
            .await?;

            let identity_domain = auth_identities::AuthIdentity {
                id: identity.id,
                user_id: identity.user_id,
                provider: identity.provider.clone(),
                provider_user_id: identity.provider_user_id.clone(),
                email: identity.email.clone(),
                password_hash: identity.password_hash.clone(),
                last_login_at: identity.last_login_at,
                created_at: identity.created_at,
                updated_at: identity.updated_at,
            };

            let updated = auth_identities::update_identity(txn, identity_domain).await?;

            assert_eq!(updated.id, identity.id);
            assert_eq!(updated.email, identity.email);
            assert_eq!(updated.provider_user_id, identity.provider_user_id);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
