//! DTOs for users_sea adapter.

/// DTO for creating a new user.
#[derive(Debug, Clone)]
pub struct UserCreate {
    pub username: String,
    pub is_ai: bool,
}

impl UserCreate {
    pub fn new(username: impl Into<String>, is_ai: bool) -> Self {
        Self {
            username: username.into(),
            is_ai,
        }
    }
}
