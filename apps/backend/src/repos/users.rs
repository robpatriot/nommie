//! User repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::users_sea as users_adapter;
use crate::errors::domain::DomainError;

/// User domain model
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub is_ai: bool,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

pub async fn create_user(
    txn: &DatabaseTransaction,
    username: &str,
    is_ai: bool,
) -> Result<User, DomainError> {
    let dto = users_adapter::UserCreate::new(username, is_ai);
    let user = users_adapter::create_user(txn, dto).await?;
    Ok(User::from(user))
}

pub async fn find_user_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
) -> Result<Option<User>, DomainError> {
    let user = users_adapter::find_user_by_id(conn, user_id).await?;
    Ok(user.map(User::from))
}

pub async fn delete_user(txn: &DatabaseTransaction, user_id: i64) -> Result<(), DomainError> {
    users_adapter::delete_user(txn, user_id).await?;
    Ok(())
}

impl From<crate::entities::users::Model> for User {
    fn from(model: crate::entities::users::Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
            is_ai: model.is_ai,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
