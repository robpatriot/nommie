//! SeaORM adapter for user repository.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, QueryFilter, Set};

use crate::db::{as_database_connection, as_database_transaction, DbConn};
use crate::entities::{user_credentials, users};
use crate::errors::domain::{ConflictKind, DomainError, InfraErrorKind};
use crate::repos::users::{User, UserCredentials, UserRepo};

/// SeaORM implementation of UserRepo.
#[derive(Debug, Default)]
pub struct UserRepoSea;

impl UserRepoSea {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UserRepo for UserRepoSea {
    async fn find_credentials_by_email(
        &self,
        conn: &dyn DbConn,
        email: &str,
    ) -> Result<Option<UserCredentials>, DomainError> {
        let credential = {
            if let Some(txn) = as_database_transaction(conn) {
                user_credentials::Entity::find()
                    .filter(user_credentials::Column::Email.eq(email))
                    .one(txn)
                    .await
            } else if let Some(db) = as_database_connection(conn) {
                user_credentials::Entity::find()
                    .filter(user_credentials::Column::Email.eq(email))
                    .one(db)
                    .await
            } else {
                return Err(DomainError::infra(
                    InfraErrorKind::Other("Connection type".to_string()),
                    "Unsupported DbConn type for SeaORM".to_string(),
                ));
            }
        }
        .map_err(|e| {
            DomainError::infra(
                InfraErrorKind::Other("Database error".to_string()),
                format!("Failed to query user credentials: {e}"),
            )
        })?;

        Ok(credential.map(|c| UserCredentials {
            id: c.id,
            user_id: c.user_id,
            password_hash: c.password_hash,
            email: c.email,
            google_sub: c.google_sub,
            last_login: c.last_login,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }))
    }

    async fn create_user(
        &self,
        conn: &dyn DbConn,
        sub: &str,
        username: &str,
        is_ai: bool,
    ) -> Result<User, DomainError> {
        let now = time::OffsetDateTime::now_utc();
        let user_active = users::ActiveModel {
            id: NotSet,
            sub: Set(sub.to_string()),
            username: Set(Some(username.to_string())),
            is_ai: Set(is_ai),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let user = if let Some(txn) = as_database_transaction(conn) {
            user_active.insert(txn).await
        } else if let Some(db) = as_database_connection(conn) {
            user_active.insert(db).await
        } else {
            return Err(DomainError::infra(
                InfraErrorKind::Other("Connection type".to_string()),
                "Unsupported DbConn type for SeaORM".to_string(),
            ));
        }
        .map_err(|e| {
            // Map unique constraint violations to specific errors
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                DomainError::conflict(
                    ConflictKind::UniqueEmail,
                    format!("User with this sub already exists: {e}"),
                )
            } else {
                DomainError::infra(
                    InfraErrorKind::Other("Database error".to_string()),
                    format!("Failed to create user: {e}"),
                )
            }
        })?;

        Ok(User {
            id: user.id,
            sub: user.sub,
            username: user.username,
            is_ai: user.is_ai,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
    }

    async fn create_credentials(
        &self,
        conn: &dyn DbConn,
        user_id: i64,
        email: &str,
        google_sub: Option<&str>,
    ) -> Result<UserCredentials, DomainError> {
        let now = time::OffsetDateTime::now_utc();
        let credential_active = user_credentials::ActiveModel {
            id: NotSet,
            user_id: Set(user_id),
            password_hash: Set(None),
            email: Set(email.to_string()),
            google_sub: Set(google_sub.map(|s| s.to_string())),
            last_login: Set(Some(now)),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let credential = if let Some(txn) = as_database_transaction(conn) {
            credential_active.insert(txn).await
        } else if let Some(db) = as_database_connection(conn) {
            credential_active.insert(db).await
        } else {
            return Err(DomainError::infra(
                InfraErrorKind::Other("Connection type".to_string()),
                "Unsupported DbConn type for SeaORM".to_string(),
            ));
        }
        .map_err(|e| {
            // Map unique constraint violations to specific errors
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                DomainError::conflict(
                    ConflictKind::UniqueEmail,
                    format!("Email already exists: {e}"),
                )
            } else {
                DomainError::infra(
                    InfraErrorKind::Other("Database error".to_string()),
                    format!("Failed to create user credentials: {e}"),
                )
            }
        })?;

        Ok(UserCredentials {
            id: credential.id,
            user_id: credential.user_id,
            password_hash: credential.password_hash,
            email: credential.email,
            google_sub: credential.google_sub,
            last_login: credential.last_login,
            created_at: credential.created_at,
            updated_at: credential.updated_at,
        })
    }

    async fn update_credentials(
        &self,
        conn: &dyn DbConn,
        credentials: UserCredentials,
    ) -> Result<UserCredentials, DomainError> {
        let mut credential_active: user_credentials::ActiveModel = credentials.into();
        credential_active.updated_at = Set(time::OffsetDateTime::now_utc());

        let credential = if let Some(txn) = as_database_transaction(conn) {
            credential_active.update(txn).await
        } else if let Some(db) = as_database_connection(conn) {
            credential_active.update(db).await
        } else {
            return Err(DomainError::infra(
                InfraErrorKind::Other("Connection type".to_string()),
                "Unsupported DbConn type for SeaORM".to_string(),
            ));
        }
        .map_err(|e| {
            // Map unique constraint violations to specific errors
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                DomainError::conflict(
                    ConflictKind::UniqueEmail,
                    format!("Email already exists: {e}"),
                )
            } else {
                DomainError::infra(
                    InfraErrorKind::Other("Database error".to_string()),
                    format!("Failed to update user credentials: {e}"),
                )
            }
        })?;

        Ok(UserCredentials {
            id: credential.id,
            user_id: credential.user_id,
            password_hash: credential.password_hash,
            email: credential.email,
            google_sub: credential.google_sub,
            last_login: credential.last_login,
            created_at: credential.created_at,
            updated_at: credential.updated_at,
        })
    }

    async fn find_user_by_id(
        &self,
        conn: &dyn DbConn,
        user_id: i64,
    ) -> Result<Option<User>, DomainError> {
        let user = {
            if let Some(txn) = as_database_transaction(conn) {
                users::Entity::find_by_id(user_id).one(txn).await
            } else if let Some(db) = as_database_connection(conn) {
                users::Entity::find_by_id(user_id).one(db).await
            } else {
                return Err(DomainError::infra(
                    InfraErrorKind::Other("Connection type".to_string()),
                    "Unsupported DbConn type for SeaORM".to_string(),
                ));
            }
        }
        .map_err(|e| {
            DomainError::infra(
                InfraErrorKind::Other("Database error".to_string()),
                format!("Failed to query user: {e}"),
            )
        })?;

        Ok(user.map(|u| User {
            id: u.id,
            sub: u.sub,
            username: u.username,
            is_ai: u.is_ai,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }))
    }
}

// Conversion from SeaORM model to domain model
impl From<user_credentials::Model> for UserCredentials {
    fn from(model: user_credentials::Model) -> Self {
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

impl From<UserCredentials> for user_credentials::ActiveModel {
    fn from(domain: UserCredentials) -> Self {
        Self {
            id: Set(domain.id),
            user_id: Set(domain.user_id),
            password_hash: Set(domain.password_hash),
            email: Set(domain.email),
            google_sub: Set(domain.google_sub),
            last_login: Set(domain.last_login),
            created_at: Set(domain.created_at),
            updated_at: Set(domain.updated_at),
        }
    }
}
