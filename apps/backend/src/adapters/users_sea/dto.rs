//! DTOs for users_sea adapter.

/// DTO for creating a new user.
#[derive(Debug, Clone)]
pub struct UserCreate {
    pub sub: String,
    pub username: String,
    pub is_ai: bool,
}

impl UserCreate {
    pub fn new(sub: impl Into<String>, username: impl Into<String>, is_ai: bool) -> Self {
        Self {
            sub: sub.into(),
            username: username.into(),
            is_ai,
        }
    }
}

/// DTO for creating new user credentials.
#[derive(Debug, Clone)]
pub struct CredentialsCreate {
    pub user_id: i64,
    pub email: String,
    pub google_sub: Option<String>,
    pub password_hash: Option<String>,
}

impl CredentialsCreate {
    pub fn new(
        user_id: i64,
        email: impl Into<String>,
        google_sub: Option<impl Into<String>>,
        password_hash: Option<impl Into<String>>,
    ) -> Self {
        Self {
            user_id,
            email: email.into(),
            google_sub: google_sub.map(|s| s.into()),
            password_hash: password_hash.map(|s| s.into()),
        }
    }
}

/// DTO for updating existing user credentials.
#[derive(Debug, Clone)]
pub struct CredentialsUpdate {
    pub id: i64,
    pub user_id: i64,
    pub email: String,
    pub google_sub: Option<String>,
    pub password_hash: Option<String>,
    pub last_login: Option<time::OffsetDateTime>,
}

impl CredentialsUpdate {
    pub fn new(
        id: i64,
        user_id: i64,
        email: impl Into<String>,
        google_sub: Option<impl Into<String>>,
        password_hash: Option<impl Into<String>>,
        last_login: Option<time::OffsetDateTime>,
    ) -> Self {
        Self {
            id,
            user_id,
            email: email.into(),
            google_sub: google_sub.map(|s| s.into()),
            password_hash: password_hash.map(|s| s.into()),
            last_login,
        }
    }
}
