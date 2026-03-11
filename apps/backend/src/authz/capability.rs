//! Admin capability vocabulary.

use crate::entities::users::UserRole;

/// Capabilities for admin-area authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdminCapability {
    /// Access to admin area (boundary check).
    AccessAdmin,
    /// Manage user roles (grant/revoke admin).
    ManageUserRoles,
    /// View system status (reserved for future use).
    ViewSystemStatus,
}

impl AdminCapability {
    /// Returns capabilities granted to the given role.
    pub fn for_role(role: UserRole) -> &'static [AdminCapability] {
        match role {
            UserRole::Admin => &[
                AdminCapability::AccessAdmin,
                AdminCapability::ManageUserRoles,
                AdminCapability::ViewSystemStatus,
            ],
            UserRole::User => &[],
        }
    }
}
