//! Policy helpers for capability checks.

use super::capability::AdminCapability;
use super::principal::Principal;

/// Returns true if the principal has the given capability.
pub fn has_capability(principal: &Principal, cap: AdminCapability) -> bool {
    AdminCapability::for_role(principal.role.clone()).contains(&cap)
}
