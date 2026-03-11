//! Request-scoped authenticated principal.

use crate::entities::users::UserRole;

/// Authenticated actor for authorization checks.
#[derive(Debug, Clone)]
pub struct Principal {
    pub user_id: i64,
    pub role: UserRole,
}
