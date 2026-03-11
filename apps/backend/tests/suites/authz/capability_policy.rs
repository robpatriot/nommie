//! Authz capability and policy tests.

use backend::authz::{has_capability, AdminCapability, Principal};
use backend::entities::users::UserRole;

#[test]
fn admin_has_all_capabilities() {
    let principal = Principal {
        user_id: 1,
        role: UserRole::Admin,
    };
    assert!(has_capability(&principal, AdminCapability::AccessAdmin));
    assert!(has_capability(&principal, AdminCapability::ManageUserRoles));
    assert!(has_capability(
        &principal,
        AdminCapability::ViewSystemStatus
    ));
}

#[test]
fn user_has_no_capabilities() {
    let principal = Principal {
        user_id: 1,
        role: UserRole::User,
    };
    assert!(!has_capability(&principal, AdminCapability::AccessAdmin));
    assert!(!has_capability(
        &principal,
        AdminCapability::ManageUserRoles
    ));
    assert!(!has_capability(
        &principal,
        AdminCapability::ViewSystemStatus
    ));
}

#[test]
fn has_capability_returns_correctly_for_each_cap() {
    let admin = Principal {
        user_id: 1,
        role: UserRole::Admin,
    };
    let user = Principal {
        user_id: 2,
        role: UserRole::User,
    };

    for cap in [
        AdminCapability::AccessAdmin,
        AdminCapability::ManageUserRoles,
        AdminCapability::ViewSystemStatus,
    ] {
        assert!(has_capability(&admin, cap), "Admin should have {cap:?}");
        assert!(!has_capability(&user, cap), "User should not have {cap:?}");
    }
}
