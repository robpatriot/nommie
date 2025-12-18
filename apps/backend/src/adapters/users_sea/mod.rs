//! SeaORM adapter for user repository.

use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, NotSet,
    QueryFilter, Set,
};

use crate::entities::{user_credentials, users};

pub mod dto;

pub use dto::{CredentialsCreate, CredentialsUpdate, UserCreate};

// Adapter functions return DbErr; repos layer maps to DomainError via From<DbErr>.

pub async fn find_credentials_by_email<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    email: &str,
) -> Result<Option<user_credentials::Model>, sea_orm::DbErr> {
    user_credentials::Entity::find()
        .filter(user_credentials::Column::Email.eq(email))
        .one(conn)
        .await
}

pub async fn create_user(
    txn: &DatabaseTransaction,
    dto: UserCreate,
) -> Result<users::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let user_active = users::ActiveModel {
        id: NotSet,
        sub: Set(dto.sub),
        username: Set(Some(dto.username)),
        is_ai: Set(dto.is_ai),
        created_at: Set(now),
        updated_at: Set(now),
    };

    user_active.insert(txn).await
}

pub async fn ensure_user_by_sub(
    txn: &DatabaseTransaction,
    dto: UserCreate,
) -> Result<(users::Model, bool), sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let sub = dto.sub.clone();

    let user_active = users::ActiveModel {
        id: NotSet,
        sub: Set(dto.sub),
        username: Set(Some(dto.username)),
        is_ai: Set(dto.is_ai),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let rows = users::Entity::insert(user_active)
        .on_conflict(
            OnConflict::column(users::Column::Sub)
                .do_nothing()
                .to_owned(),
        )
        .exec_without_returning(txn)
        .await?;

    let inserted = rows == 1;
    let user = users::Entity::find()
        .filter(users::Column::Sub.eq(sub))
        .one(txn)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("users.sub not found".to_string()))?;

    Ok((user, inserted))
}

pub async fn create_credentials(
    txn: &DatabaseTransaction,
    dto: CredentialsCreate,
) -> Result<user_credentials::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let credential_active = user_credentials::ActiveModel {
        id: NotSet,
        user_id: Set(dto.user_id),
        password_hash: Set(dto.password_hash),
        email: Set(dto.email),
        google_sub: Set(dto.google_sub),
        last_login: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
    };

    credential_active.insert(txn).await
}

pub async fn ensure_credentials_by_email(
    txn: &DatabaseTransaction,
    dto: CredentialsCreate,
) -> Result<(user_credentials::Model, bool), sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let email = dto.email.clone();
    let credential_active = user_credentials::ActiveModel {
        id: NotSet,
        user_id: Set(dto.user_id),
        password_hash: Set(dto.password_hash),
        email: Set(dto.email),
        google_sub: Set(dto.google_sub),
        last_login: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let rows = user_credentials::Entity::insert(credential_active)
        .on_conflict(
            OnConflict::column(user_credentials::Column::Email)
                .do_nothing()
                .to_owned(),
        )
        .exec_without_returning(txn)
        .await?;

    let inserted = rows == 1;
    let creds = user_credentials::Entity::find()
        .filter(user_credentials::Column::Email.eq(email))
        .one(txn)
        .await?
        .ok_or_else(|| {
            sea_orm::DbErr::RecordNotFound("user_credentials.email not found".to_string())
        })?;

    Ok((creds, inserted))
}

pub async fn update_credentials(
    txn: &DatabaseTransaction,
    dto: CredentialsUpdate,
) -> Result<user_credentials::Model, sea_orm::DbErr> {
    let credentials = user_credentials::ActiveModel {
        id: Set(dto.id),
        user_id: Set(dto.user_id),
        password_hash: Set(dto.password_hash),
        email: Set(dto.email),
        google_sub: Set(dto.google_sub),
        last_login: Set(dto.last_login),
        created_at: NotSet,
        updated_at: Set(time::OffsetDateTime::now_utc()),
    };
    credentials.update(txn).await
}

pub async fn find_user_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
) -> Result<Option<users::Model>, sea_orm::DbErr> {
    users::Entity::find_by_id(user_id).one(conn).await
}

pub async fn find_user_by_sub<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    sub: &str,
) -> Result<Option<users::Model>, sea_orm::DbErr> {
    users::Entity::find()
        .filter(users::Column::Sub.eq(sub))
        .one(conn)
        .await
}

pub async fn delete_user(txn: &DatabaseTransaction, user_id: i64) -> Result<(), sea_orm::DbErr> {
    users::Entity::delete_by_id(user_id).exec(txn).await?;
    Ok(())
}
