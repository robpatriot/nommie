//! DTOs for users_sea adapter.

use crate::entities::users::UserRole;

/// DTO for creating a new user.
#[derive(Debug, Clone)]
pub struct UserCreate {
    pub username: String,
    pub is_ai: bool,
    pub role: UserRole,
}

impl UserCreate {
    pub fn new(username: impl Into<String>, is_ai: bool, role: UserRole) -> Self {
        Self {
            username: username.into(),
            is_ai,
            role,
        }
    }
}
