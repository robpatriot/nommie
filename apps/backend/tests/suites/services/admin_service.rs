//! AdminService tests.

use backend::authz::Principal;
use backend::db::txn::with_txn;
use backend::entities::users::UserRole;
use backend::errors::domain::{ConflictKind, DomainError, NotFoundKind};
use backend::services::admin::{AdminService, RoleMutationRequest};
use backend::AppError;
use backend_test_support::unique_helpers::{unique_email, unique_str};

use crate::support::build_test_state;
use crate::support::factory::{seed_user_with_sub, seed_user_with_sub_and_role};

fn admin_principal(user_id: i64) -> Principal {
    Principal {
        user_id,
        role: UserRole::Admin,
    }
}

#[tokio::test]
async fn grant_admin_success() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let admin = seed_user_with_sub_and_role(
                txn,
                &unique_str("grant-admin-actor"),
                Some(&unique_email("grant-admin-actor")),
                UserRole::Admin,
            )
            .await?;
            let target = seed_user_with_sub(
                txn,
                &unique_str("grant-admin-target"),
                Some(&unique_email("grant-admin-target")),
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest { reason: None };
            let resp = service
                .grant_admin(txn, &admin_principal(admin.id), target.id, &req)
                .await?;

            assert_eq!(resp.user.id, target.id);
            assert_eq!(resp.user.role, UserRole::Admin);
            assert!(resp.changed);

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn grant_admin_idempotent() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let admin = seed_user_with_sub_and_role(
                txn,
                &unique_str("grant-admin-idem-actor"),
                Some(&unique_email("grant-admin-idem-actor")),
                UserRole::Admin,
            )
            .await?;
            let target = seed_user_with_sub_and_role(
                txn,
                &unique_str("grant-admin-idem-target"),
                Some(&unique_email("grant-admin-idem-target")),
                UserRole::Admin,
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest { reason: None };
            let resp = service
                .grant_admin(txn, &admin_principal(admin.id), target.id, &req)
                .await?;

            assert_eq!(resp.user.id, target.id);
            assert_eq!(resp.user.role, UserRole::Admin);
            assert!(!resp.changed);

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn grant_admin_target_not_found() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let admin = seed_user_with_sub_and_role(
                txn,
                &unique_str("grant-admin-404-actor"),
                Some(&unique_email("grant-admin-404-actor")),
                UserRole::Admin,
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest { reason: None };
            let err = service
                .grant_admin(txn, &admin_principal(admin.id), 999999999, &req)
                .await
                .map_err(|e| e.to_string());

            assert!(err.is_err());
            let err = err.unwrap_err();
            assert!(err.contains("Target user not found"));

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn revoke_admin_success() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let admin = seed_user_with_sub_and_role(
                txn,
                &unique_str("revoke-admin-actor"),
                Some(&unique_email("revoke-admin-actor")),
                UserRole::Admin,
            )
            .await?;
            let target = seed_user_with_sub_and_role(
                txn,
                &unique_str("revoke-admin-target"),
                Some(&unique_email("revoke-admin-target")),
                UserRole::Admin,
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest { reason: None };
            let resp = service
                .revoke_admin(txn, &admin_principal(admin.id), target.id, &req)
                .await?;

            assert_eq!(resp.user.id, target.id);
            assert_eq!(resp.user.role, UserRole::User);
            assert!(resp.changed);

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn revoke_admin_idempotent() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let admin = seed_user_with_sub_and_role(
                txn,
                &unique_str("revoke-admin-idem-actor"),
                Some(&unique_email("revoke-admin-idem-actor")),
                UserRole::Admin,
            )
            .await?;
            let target = seed_user_with_sub(
                txn,
                &unique_str("revoke-admin-idem-target"),
                Some(&unique_email("revoke-admin-idem-target")),
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest { reason: None };
            let resp = service
                .revoke_admin(txn, &admin_principal(admin.id), target.id, &req)
                .await?;

            assert_eq!(resp.user.id, target.id);
            assert_eq!(resp.user.role, UserRole::User);
            assert!(!resp.changed);

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn revoke_admin_self_revoke_rejected() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let admin = seed_user_with_sub_and_role(
                txn,
                &unique_str("revoke-admin-self"),
                Some(&unique_email("revoke-admin-self")),
                UserRole::Admin,
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest { reason: None };
            let err = service
                .revoke_admin(txn, &admin_principal(admin.id), admin.id, &req)
                .await;

            assert!(err.is_err());
            let domain_err = err.unwrap_err();
            match &domain_err {
                DomainError::Conflict(ConflictKind::CannotRevokeOwnAdmin, _) => {}
                _ => panic!("expected CannotRevokeOwnAdmin, got {:?}", domain_err),
            }

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn revoke_admin_last_admin_rejected() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let actor = seed_user_with_sub(
                txn,
                &unique_str("revoke-admin-last-actor"),
                Some(&unique_email("revoke-admin-last-actor")),
            )
            .await?;
            let target = seed_user_with_sub_and_role(
                txn,
                &unique_str("revoke-admin-last-target"),
                Some(&unique_email("revoke-admin-last-target")),
                UserRole::Admin,
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest { reason: None };
            let err = service
                .revoke_admin(txn, &admin_principal(actor.id), target.id, &req)
                .await;

            assert!(err.is_err());
            let domain_err = err.unwrap_err();
            match &domain_err {
                DomainError::Conflict(ConflictKind::LastAdminProtection, _) => {}
                _ => panic!("expected LastAdminProtection, got {:?}", domain_err),
            }

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn revoke_admin_target_not_found() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let admin = seed_user_with_sub_and_role(
                txn,
                &unique_str("revoke-admin-404-actor"),
                Some(&unique_email("revoke-admin-404-actor")),
                UserRole::Admin,
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest { reason: None };
            let err = service
                .revoke_admin(txn, &admin_principal(admin.id), 999999999, &req)
                .await;

            assert!(err.is_err());
            let domain_err = err.unwrap_err();
            match &domain_err {
                DomainError::NotFound(NotFoundKind::TargetUser, _) => {}
                _ => panic!("expected TargetUser not found, got {:?}", domain_err),
            }

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}
