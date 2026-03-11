//! Admin audit logging tests.

use backend::db::txn::with_txn;
use backend::entities::admin_audit_log;
use backend::entities::users::UserRole;
use backend::services::admin::{AdminService, RoleMutationRequest};
use backend_test_support::unique_helpers::{unique_email, unique_str};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::support::build_test_state;
use crate::support::factory::{seed_user_with_sub, seed_user_with_sub_and_role};

#[tokio::test]
async fn grant_success_writes_audit_with_result_success() -> Result<(), Box<dyn std::error::Error>>
{
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let admin = seed_user_with_sub_and_role(
                txn,
                &unique_str("audit-grant-actor"),
                Some(&unique_email("audit-grant-actor")),
                UserRole::Admin,
            )
            .await?;
            let target = seed_user_with_sub(
                txn,
                &unique_str("audit-grant-target"),
                Some(&unique_email("audit-grant-target")),
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest {
                reason: Some("test reason".into()),
            };
            service
                .grant_admin(txn, &admin_principal(admin.id), target.id, &req)
                .await
                .map_err(backend::AppError::from)?;

            let entries = admin_audit_log::Entity::find()
                .filter(admin_audit_log::Column::Action.eq("grant_admin"))
                .filter(admin_audit_log::Column::TargetId.eq(target.id))
                .all(txn)
                .await?;

            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].result, "success");
            assert_eq!(entries[0].actor_user_id, admin.id);
            assert_eq!(entries[0].reason.as_deref(), Some("test reason"));

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn grant_noop_writes_audit_with_result_noop() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let admin = seed_user_with_sub_and_role(
                txn,
                &unique_str("audit-grant-noop-actor"),
                Some(&unique_email("audit-grant-noop-actor")),
                UserRole::Admin,
            )
            .await?;
            let target = seed_user_with_sub_and_role(
                txn,
                &unique_str("audit-grant-noop-target"),
                Some(&unique_email("audit-grant-noop-target")),
                UserRole::Admin,
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest { reason: None };
            service
                .grant_admin(txn, &admin_principal(admin.id), target.id, &req)
                .await
                .map_err(backend::AppError::from)?;

            let entries = admin_audit_log::Entity::find()
                .filter(admin_audit_log::Column::Action.eq("grant_admin"))
                .filter(admin_audit_log::Column::TargetId.eq(target.id))
                .all(txn)
                .await?;

            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].result, "noop");

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn revoke_success_writes_audit() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let admin = seed_user_with_sub_and_role(
                txn,
                &unique_str("audit-revoke-actor"),
                Some(&unique_email("audit-revoke-actor")),
                UserRole::Admin,
            )
            .await?;
            let target = seed_user_with_sub_and_role(
                txn,
                &unique_str("audit-revoke-target"),
                Some(&unique_email("audit-revoke-target")),
                UserRole::Admin,
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest { reason: None };
            service
                .revoke_admin(txn, &admin_principal(admin.id), target.id, &req)
                .await
                .map_err(backend::AppError::from)?;

            let entries = admin_audit_log::Entity::find()
                .filter(admin_audit_log::Column::Action.eq("revoke_admin"))
                .filter(admin_audit_log::Column::TargetId.eq(target.id))
                .all(txn)
                .await?;

            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].result, "success");

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn revoke_self_revoke_writes_audit_with_result_rejected(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let admin = seed_user_with_sub_and_role(
                txn,
                &unique_str("audit-revoke-self"),
                Some(&unique_email("audit-revoke-self")),
                UserRole::Admin,
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest { reason: None };
            let _ = service
                .revoke_admin(txn, &admin_principal(admin.id), admin.id, &req)
                .await;

            let entries = admin_audit_log::Entity::find()
                .filter(admin_audit_log::Column::Action.eq("revoke_admin"))
                .filter(admin_audit_log::Column::TargetId.eq(admin.id))
                .all(txn)
                .await?;

            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].result, "rejected");

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn revoke_last_admin_writes_audit_with_result_rejected(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let actor = seed_user_with_sub(
                txn,
                &unique_str("audit-revoke-last-actor"),
                Some(&unique_email("audit-revoke-last-actor")),
            )
            .await?;
            let target = seed_user_with_sub_and_role(
                txn,
                &unique_str("audit-revoke-last"),
                Some(&unique_email("audit-revoke-last")),
                UserRole::Admin,
            )
            .await?;

            let service = AdminService;
            let req = RoleMutationRequest { reason: None };
            let _ = service
                .revoke_admin(txn, &admin_principal(actor.id), target.id, &req)
                .await;

            let entries = admin_audit_log::Entity::find()
                .filter(admin_audit_log::Column::Action.eq("revoke_admin"))
                .filter(admin_audit_log::Column::TargetId.eq(target.id))
                .all(txn)
                .await?;

            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].result, "rejected");

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

fn admin_principal(user_id: i64) -> backend::authz::Principal {
    backend::authz::Principal {
        user_id,
        role: UserRole::Admin,
    }
}
