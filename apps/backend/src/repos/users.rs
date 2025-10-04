//! User repository trait for domain layer.

use async_trait::async_trait;

use crate::db::DbConn;
use crate::errors::domain::DomainError;

/// User domain model
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: i64,
    pub sub: String,
    pub username: Option<String>,
    pub is_ai: bool,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

/// User credentials domain model
#[derive(Debug, Clone, PartialEq)]
pub struct UserCredentials {
    pub id: i64,
    pub user_id: i64,
    pub password_hash: Option<String>,
    pub email: String,
    pub google_sub: Option<String>,
    pub last_login: Option<time::OffsetDateTime>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

/// Repository trait for user operations.
/// 
/// This trait is domain-facing and contains no SeaORM imports.
/// Adapters implement this trait using SeaORM entities.
#[async_trait]
pub trait UserRepo: Send + Sync {
    /// Find user credentials by email
    async fn find_credentials_by_email(
        &self,
        conn: &dyn DbConn,
        email: &str,
    ) -> Result<Option<UserCredentials>, DomainError>;

    /// Create a new user
    async fn create_user(
        &self,
        conn: &dyn DbConn,
        sub: &str,
        username: &str,
        is_ai: bool,
    ) -> Result<User, DomainError>;

    /// Create user credentials
    async fn create_credentials(
        &self,
        conn: &dyn DbConn,
        user_id: i64,
        email: &str,
        google_sub: Option<&str>,
    ) -> Result<UserCredentials, DomainError>;

    /// Update user credentials
    async fn update_credentials(
        &self,
        conn: &dyn DbConn,
        credentials: UserCredentials,
    ) -> Result<UserCredentials, DomainError>;

    /// Find user by ID
    async fn find_user_by_id(&self, conn: &dyn DbConn, user_id: i64) -> Result<Option<User>, DomainError>;
}
