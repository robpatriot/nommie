//! SeaORM adapter for user repository - generic over ConnectionTrait.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, NotSet, QueryFilter, Set,
};

use crate::entities::{user_credentials, users};

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

pub async fn create_user<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    sub: &str,
    username: &str,
    is_ai: bool,
) -> Result<users::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let user_active = users::ActiveModel {
        id: NotSet,
        sub: Set(sub.to_string()),
        username: Set(Some(username.to_string())),
        is_ai: Set(is_ai),
        created_at: Set(now),
        updated_at: Set(now),
    };

    user_active.insert(conn).await
}

pub async fn create_credentials<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
    email: &str,
    google_sub: Option<&str>,
) -> Result<user_credentials::Model, sea_orm::DbErr> {
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

    credential_active.insert(conn).await
}

pub async fn update_credentials<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    mut credentials: user_credentials::ActiveModel,
) -> Result<user_credentials::Model, sea_orm::DbErr> {
    credentials.updated_at = Set(time::OffsetDateTime::now_utc());
    credentials.update(conn).await
}

pub async fn find_user_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
) -> Result<Option<users::Model>, sea_orm::DbErr> {
    users::Entity::find_by_id(user_id).one(conn).await
}
