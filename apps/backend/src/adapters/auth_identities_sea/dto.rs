//! DTOs for auth_identities_sea adapter.

use time::OffsetDateTime;

/// DTO for creating a new auth identity.
#[derive(Debug, Clone)]
pub struct IdentityCreate {
    pub user_id: i64,
    pub provider: String,
    pub provider_user_id: String,
    pub email: String,
    pub password_hash: Option<String>,
}

impl IdentityCreate {
    pub fn new(
        user_id: i64,
        provider: impl Into<String>,
        provider_user_id: impl Into<String>,
        email: impl Into<String>,
    ) -> Self {
        Self {
            user_id,
            provider: provider.into(),
            provider_user_id: provider_user_id.into(),
            email: email.into(),
            password_hash: None,
        }
    }

    pub fn with_password_hash(mut self, password_hash: impl Into<String>) -> Self {
        self.password_hash = Some(password_hash.into());
        self
    }
}

/// DTO for updating an existing auth identity.
#[derive(Debug, Clone)]
pub struct IdentityUpdate {
    pub id: i64,
    pub user_id: i64,
    pub provider: String,
    pub provider_user_id: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub last_login_at: Option<OffsetDateTime>,
}

impl IdentityUpdate {
    pub fn new(
        id: i64,
        user_id: i64,
        provider: impl Into<String>,
        provider_user_id: impl Into<String>,
        email: impl Into<String>,
    ) -> Self {
        Self {
            id,
            user_id,
            provider: provider.into(),
            provider_user_id: provider_user_id.into(),
            email: email.into(),
            password_hash: None,
            last_login_at: None,
        }
    }

    pub fn with_password_hash(mut self, password_hash: impl Into<String>) -> Self {
        self.password_hash = Some(password_hash.into());
        self
    }

    pub fn with_last_login_at(mut self, last_login_at: OffsetDateTime) -> Self {
        self.last_login_at = Some(last_login_at);
        self
    }

    pub fn with_password_hash_opt(mut self, password_hash: Option<String>) -> Self {
        self.password_hash = password_hash;
        self
    }

    pub fn with_last_login_at_opt(mut self, last_login_at: Option<OffsetDateTime>) -> Self {
        self.last_login_at = last_login_at;
        self
    }
}
