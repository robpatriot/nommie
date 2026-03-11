//! Auth identities repository for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::auth_identities_sea as auth_identities_adapter;
use crate::errors::domain::DomainError;

/// Auth identity domain model
#[derive(Debug, Clone, PartialEq)]
pub struct AuthIdentity {
    pub id: i64,
    pub user_id: i64,
    pub provider: String,
    pub provider_user_id: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub last_login_at: Option<time::OffsetDateTime>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

pub async fn find_by_provider_user_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    provider: &str,
    provider_user_id: &str,
) -> Result<Option<AuthIdentity>, DomainError> {
    let identity =
        auth_identities_adapter::find_by_provider_user_id(conn, provider, provider_user_id).await?;
    Ok(identity.map(AuthIdentity::from))
}

pub async fn find_by_provider_email<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    provider: &str,
    email: &str,
) -> Result<Option<AuthIdentity>, DomainError> {
    let identity = auth_identities_adapter::find_by_provider_email(conn, provider, email).await?;
    Ok(identity.map(AuthIdentity::from))
}

pub async fn find_email_by_user_and_provider<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
    provider: &str,
) -> Result<Option<String>, DomainError> {
    auth_identities_adapter::find_email_by_user_and_provider(conn, user_id, provider)
        .await
        .map_err(DomainError::from)
}

pub async fn create_identity(
    txn: &DatabaseTransaction,
    user_id: i64,
    provider: &str,
    provider_user_id: &str,
    email: &str,
) -> Result<AuthIdentity, DomainError> {
    let dto =
        auth_identities_adapter::IdentityCreate::new(user_id, provider, provider_user_id, email);
    let identity = auth_identities_adapter::create_identity(txn, dto).await?;
    Ok(AuthIdentity::from(identity))
}

pub async fn ensure_identity_by_provider_user_id(
    txn: &DatabaseTransaction,
    user_id: i64,
    provider: &str,
    provider_user_id: &str,
    email: &str,
) -> Result<(AuthIdentity, bool), DomainError> {
    let dto =
        auth_identities_adapter::IdentityCreate::new(user_id, provider, provider_user_id, email);
    let (identity, inserted) =
        auth_identities_adapter::ensure_identity_by_provider_user_id(txn, dto).await?;
    Ok((AuthIdentity::from(identity), inserted))
}

pub async fn update_identity(
    txn: &DatabaseTransaction,
    identity: AuthIdentity,
) -> Result<AuthIdentity, DomainError> {
    let dto = auth_identities_adapter::IdentityUpdate::new(
        identity.id,
        identity.user_id,
        identity.provider,
        identity.provider_user_id,
        identity.email,
    )
    .with_password_hash_opt(identity.password_hash)
    .with_last_login_at_opt(identity.last_login_at);
    let updated = auth_identities_adapter::update_identity(txn, dto).await?;
    Ok(AuthIdentity::from(updated))
}

impl From<crate::entities::user_auth_identities::Model> for AuthIdentity {
    fn from(model: crate::entities::user_auth_identities::Model) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            provider: model.provider,
            provider_user_id: model.provider_user_id,
            email: model.email,
            password_hash: model.password_hash,
            last_login_at: model.last_login_at,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
