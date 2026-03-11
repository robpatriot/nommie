//! SeaORM adapter for auth identities.

use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, NotSet,
    QueryFilter, Set,
};

use crate::entities::user_auth_identities;

pub mod dto;

pub use dto::{IdentityCreate, IdentityUpdate};

// Adapter functions return DbErr; repos layer maps to DomainError via From<DbErr>.

pub async fn find_by_provider_user_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    provider: &str,
    provider_user_id: &str,
) -> Result<Option<user_auth_identities::Model>, sea_orm::DbErr> {
    user_auth_identities::Entity::find()
        .filter(user_auth_identities::Column::Provider.eq(provider))
        .filter(user_auth_identities::Column::ProviderUserId.eq(provider_user_id))
        .one(conn)
        .await
}

pub async fn find_by_provider_email<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    provider: &str,
    email: &str,
) -> Result<Option<user_auth_identities::Model>, sea_orm::DbErr> {
    user_auth_identities::Entity::find()
        .filter(user_auth_identities::Column::Provider.eq(provider))
        .filter(user_auth_identities::Column::Email.eq(email))
        .one(conn)
        .await
}

pub async fn find_email_by_user_and_provider<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
    provider: &str,
) -> Result<Option<String>, sea_orm::DbErr> {
    user_auth_identities::Entity::find()
        .filter(user_auth_identities::Column::UserId.eq(user_id))
        .filter(user_auth_identities::Column::Provider.eq(provider))
        .one(conn)
        .await
        .map(|opt| opt.map(|m| m.email))
}

pub async fn create_identity(
    txn: &DatabaseTransaction,
    dto: IdentityCreate,
) -> Result<user_auth_identities::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let active = user_auth_identities::ActiveModel {
        id: NotSet,
        user_id: Set(dto.user_id),
        provider: Set(dto.provider),
        provider_user_id: Set(dto.provider_user_id),
        email: Set(dto.email),
        password_hash: Set(dto.password_hash),
        last_login_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
    };

    active.insert(txn).await
}

pub async fn ensure_identity_by_provider_user_id(
    txn: &DatabaseTransaction,
    dto: IdentityCreate,
) -> Result<(user_auth_identities::Model, bool), sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let provider = dto.provider.clone();
    let provider_user_id = dto.provider_user_id.clone();

    let active = user_auth_identities::ActiveModel {
        id: NotSet,
        user_id: Set(dto.user_id),
        provider: Set(dto.provider),
        provider_user_id: Set(dto.provider_user_id),
        email: Set(dto.email),
        password_hash: Set(dto.password_hash),
        last_login_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let rows = user_auth_identities::Entity::insert(active)
        .on_conflict(
            OnConflict::columns([
                user_auth_identities::Column::Provider,
                user_auth_identities::Column::ProviderUserId,
            ])
            .do_nothing()
            .to_owned(),
        )
        .exec_without_returning(txn)
        .await?;

    let inserted = rows == 1;
    let identity = user_auth_identities::Entity::find()
        .filter(user_auth_identities::Column::Provider.eq(&provider))
        .filter(user_auth_identities::Column::ProviderUserId.eq(&provider_user_id))
        .one(txn)
        .await?
        .ok_or_else(|| {
            sea_orm::DbErr::RecordNotFound(
                "user_auth_identities.(provider, provider_user_id) not found".to_string(),
            )
        })?;

    Ok((identity, inserted))
}

pub async fn update_identity(
    txn: &DatabaseTransaction,
    dto: IdentityUpdate,
) -> Result<user_auth_identities::Model, sea_orm::DbErr> {
    let active = user_auth_identities::ActiveModel {
        id: Set(dto.id),
        user_id: Set(dto.user_id),
        provider: Set(dto.provider),
        provider_user_id: Set(dto.provider_user_id),
        email: Set(dto.email),
        password_hash: Set(dto.password_hash),
        last_login_at: Set(dto.last_login_at),
        created_at: NotSet,
        updated_at: Set(time::OffsetDateTime::now_utc()),
    };

    active.update(txn).await
}
