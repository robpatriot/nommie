//! User repository functions for domain layer (generic over ConnectionTrait).

use sea_orm::ConnectionTrait;

use crate::adapters::users_sea as users_adapter;
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

// Free functions (generic) mirroring the previous trait methods

pub async fn find_credentials_by_email<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    email: &str,
) -> Result<Option<UserCredentials>, DomainError> {
    let credential = users_adapter::find_credentials_by_email(conn, email).await?;
    Ok(credential.map(UserCredentials::from))
}

pub async fn create_user<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    sub: &str,
    username: &str,
    is_ai: bool,
) -> Result<User, DomainError> {
    let user = users_adapter::create_user(conn, sub, username, is_ai).await?;
    Ok(User::from(user))
}

pub async fn create_credentials<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
    email: &str,
    google_sub: Option<&str>,
) -> Result<UserCredentials, DomainError> {
    let credential = users_adapter::create_credentials(conn, user_id, email, google_sub).await?;
    Ok(UserCredentials::from(credential))
}

pub async fn update_credentials<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    credentials: UserCredentials,
) -> Result<UserCredentials, DomainError> {
    let credential = users_adapter::update_credentials(
        conn,
        credentials.id,
        credentials.user_id,
        credentials.password_hash,
        credentials.email,
        credentials.google_sub,
        credentials.last_login,
        credentials.created_at,
    )
    .await?;
    Ok(UserCredentials::from(credential))
}

pub async fn find_user_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
) -> Result<Option<User>, DomainError> {
    let user = users_adapter::find_user_by_id(conn, user_id).await?;
    Ok(user.map(User::from))
}

// Conversions between SeaORM models and domain models

impl From<crate::entities::users::Model> for User {
    fn from(model: crate::entities::users::Model) -> Self {
        Self {
            id: model.id,
            sub: model.sub,
            username: model.username,
            is_ai: model.is_ai,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

impl From<crate::entities::user_credentials::Model> for UserCredentials {
    fn from(model: crate::entities::user_credentials::Model) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            password_hash: model.password_hash,
            email: model.email,
            google_sub: model.google_sub,
            last_login: model.last_login,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
