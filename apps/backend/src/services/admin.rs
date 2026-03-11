//! Admin service for user role management.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::admin_audit_log_sea;
use crate::authz::{has_capability, AdminCapability, Principal};
use crate::entities::users::UserRole;
use crate::errors::domain::{ConflictKind, DomainError, InfraErrorKind, NotFoundKind};
use crate::repos::admin_users::{self, AdminUserSearchQuery, AdminUserSearchResult};
use crate::repos::{admin_audit_log, auth_identities, users};

const PROVIDER_GOOGLE: &str = "google";

/// Admin user summary for API responses.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminUserSummary {
    pub id: i64,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: UserRole,
}

/// Request body for role mutation (grant/revoke).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RoleMutationRequest {
    pub reason: Option<String>,
}

/// Response for grant/revoke.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RoleMutationResponse {
    pub user: AdminUserSummary,
    pub changed: bool,
}

#[derive(Default)]
pub struct AdminService;

impl AdminService {
    /// Search users for admin role management. Requires ManageUserRoles.
    pub async fn search_users<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        actor: &Principal,
        query: AdminUserSearchQuery,
    ) -> Result<AdminUserSearchResult, DomainError> {
        if !has_capability(actor, AdminCapability::ManageUserRoles) {
            return Err(DomainError::permission_denied("ManageUserRoles required"));
        }
        admin_users::search_users_for_admin(conn, query).await
    }

    /// Grant admin role to target user. Idempotent.
    pub async fn grant_admin(
        &self,
        txn: &DatabaseTransaction,
        actor: &Principal,
        target_user_id: i64,
        request: &RoleMutationRequest,
    ) -> Result<RoleMutationResponse, DomainError> {
        if !has_capability(actor, AdminCapability::ManageUserRoles) {
            admin_audit_log::insert_entry(
                txn,
                admin_audit_log_sea::AuditEntryInsert {
                    actor_user_id: actor.user_id,
                    action: "grant_admin".to_string(),
                    target_type: "user".to_string(),
                    target_id: target_user_id,
                    result: "rejected".to_string(),
                    reason: request.reason.clone(),
                    metadata_json: Some(serde_json::json!({
                        "rejection_code": "PERMISSION_DENIED"
                    })),
                },
            )
            .await?;
            return Err(DomainError::permission_denied("ManageUserRoles required"));
        }

        let target = users::find_user_by_id(txn, target_user_id)
            .await?
            .ok_or_else(|| {
                DomainError::not_found(NotFoundKind::TargetUser, "Target user not found")
            })?;

        if target.role == UserRole::Admin {
            admin_audit_log::insert_entry(
                txn,
                admin_audit_log_sea::AuditEntryInsert {
                    actor_user_id: actor.user_id,
                    action: "grant_admin".to_string(),
                    target_type: "user".to_string(),
                    target_id: target_user_id,
                    result: "noop".to_string(),
                    reason: request.reason.clone(),
                    metadata_json: Some(serde_json::json!({
                        "previous_role": "admin",
                        "new_role": "admin",
                        "changed": false
                    })),
                },
            )
            .await?;
            return Ok(RoleMutationResponse {
                user: user_to_summary(txn, &target).await?,
                changed: false,
            });
        }

        let prev_role_str = match target.role {
            UserRole::Admin => "admin",
            UserRole::User => "user",
        };
        users::update_user_role(txn, target_user_id, UserRole::Admin).await?;
        let updated = users::find_user_by_id(txn, target_user_id)
            .await?
            .ok_or_else(|| {
                DomainError::Infra(
                    InfraErrorKind::DataCorruption,
                    "user not found after role update".into(),
                )
            })?;

        admin_audit_log::insert_entry(
            txn,
            admin_audit_log_sea::AuditEntryInsert {
                actor_user_id: actor.user_id,
                action: "grant_admin".to_string(),
                target_type: "user".to_string(),
                target_id: target_user_id,
                result: "success".to_string(),
                reason: request.reason.clone(),
                metadata_json: Some(serde_json::json!({
                    "previous_role": prev_role_str,
                    "new_role": "admin",
                    "changed": true
                })),
            },
        )
        .await?;

        Ok(RoleMutationResponse {
            user: user_to_summary(txn, &updated).await?,
            changed: true,
        })
    }

    /// Revoke admin role from target user. Idempotent. Rejects self-revoke and last-admin.
    pub async fn revoke_admin(
        &self,
        txn: &DatabaseTransaction,
        actor: &Principal,
        target_user_id: i64,
        request: &RoleMutationRequest,
    ) -> Result<RoleMutationResponse, DomainError> {
        if !has_capability(actor, AdminCapability::ManageUserRoles) {
            admin_audit_log::insert_entry(
                txn,
                admin_audit_log_sea::AuditEntryInsert {
                    actor_user_id: actor.user_id,
                    action: "revoke_admin".to_string(),
                    target_type: "user".to_string(),
                    target_id: target_user_id,
                    result: "rejected".to_string(),
                    reason: request.reason.clone(),
                    metadata_json: Some(serde_json::json!({
                        "rejection_code": "PERMISSION_DENIED"
                    })),
                },
            )
            .await?;
            return Err(DomainError::permission_denied("ManageUserRoles required"));
        }

        let target = users::find_user_by_id(txn, target_user_id)
            .await?
            .ok_or_else(|| {
                DomainError::not_found(NotFoundKind::TargetUser, "Target user not found")
            })?;

        if actor.user_id == target_user_id {
            admin_audit_log::insert_entry(
                txn,
                admin_audit_log_sea::AuditEntryInsert {
                    actor_user_id: actor.user_id,
                    action: "revoke_admin".to_string(),
                    target_type: "user".to_string(),
                    target_id: target_user_id,
                    result: "rejected".to_string(),
                    reason: request.reason.clone(),
                    metadata_json: Some(serde_json::json!({
                        "previous_role": "admin",
                        "new_role": "admin",
                        "changed": false,
                        "rejection_code": "CANNOT_REVOKE_OWN_ADMIN"
                    })),
                },
            )
            .await?;
            return Err(DomainError::conflict(
                ConflictKind::CannotRevokeOwnAdmin,
                "Cannot revoke admin from yourself",
            ));
        }

        if target.role != UserRole::Admin {
            admin_audit_log::insert_entry(
                txn,
                admin_audit_log_sea::AuditEntryInsert {
                    actor_user_id: actor.user_id,
                    action: "revoke_admin".to_string(),
                    target_type: "user".to_string(),
                    target_id: target_user_id,
                    result: "noop".to_string(),
                    reason: request.reason.clone(),
                    metadata_json: Some(serde_json::json!({
                        "previous_role": "user",
                        "new_role": "user",
                        "changed": false
                    })),
                },
            )
            .await?;
            return Ok(RoleMutationResponse {
                user: user_to_summary(txn, &target).await?,
                changed: false,
            });
        }

        let remaining_admins = users::count_admins_excluding_user(txn, target_user_id).await?;
        if remaining_admins == 0 {
            admin_audit_log::insert_entry(
                txn,
                admin_audit_log_sea::AuditEntryInsert {
                    actor_user_id: actor.user_id,
                    action: "revoke_admin".to_string(),
                    target_type: "user".to_string(),
                    target_id: target_user_id,
                    result: "rejected".to_string(),
                    reason: request.reason.clone(),
                    metadata_json: Some(serde_json::json!({
                        "previous_role": "admin",
                        "new_role": "admin",
                        "changed": false,
                        "rejection_code": "LAST_ADMIN_PROTECTION"
                    })),
                },
            )
            .await?;
            return Err(DomainError::conflict(
                ConflictKind::LastAdminProtection,
                "Cannot revoke the last admin",
            ));
        }

        users::update_user_role(txn, target_user_id, UserRole::User).await?;
        let updated = users::find_user_by_id(txn, target_user_id)
            .await?
            .ok_or_else(|| {
                DomainError::Infra(
                    InfraErrorKind::DataCorruption,
                    "user not found after role update".into(),
                )
            })?;

        admin_audit_log::insert_entry(
            txn,
            admin_audit_log_sea::AuditEntryInsert {
                actor_user_id: actor.user_id,
                action: "revoke_admin".to_string(),
                target_type: "user".to_string(),
                target_id: target_user_id,
                result: "success".to_string(),
                reason: request.reason.clone(),
                metadata_json: Some(serde_json::json!({
                    "previous_role": "admin",
                    "new_role": "user",
                    "changed": true
                })),
            },
        )
        .await?;

        Ok(RoleMutationResponse {
            user: user_to_summary(txn, &updated).await?,
            changed: true,
        })
    }
}

async fn user_to_summary<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user: &users::User,
) -> Result<AdminUserSummary, DomainError> {
    let email =
        auth_identities::find_email_by_user_and_provider(conn, user.id, PROVIDER_GOOGLE).await?;
    // Display name: username preferred, fallback to email (matches search results)
    let display_name = user.username.clone().or(email.clone());
    Ok(AdminUserSummary {
        id: user.id,
        display_name,
        email,
        role: user.role.clone(),
    })
}
