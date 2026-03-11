//! SeaORM adapter for user repository.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, NotSet,
    PaginatorTrait, QueryFilter, Set,
};

use crate::entities::users;

pub mod dto;

pub use dto::UserCreate;

// Adapter functions return DbErr; repos layer maps to DomainError via From<DbErr>.

pub async fn update_user_role(
    txn: &DatabaseTransaction,
    user_id: i64,
    role: users::UserRole,
) -> Result<users::Model, sea_orm::DbErr> {
    let user = users::Entity::find_by_id(user_id)
        .one(txn)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("User not found".to_string()))?;

    let mut active: users::ActiveModel = user.into();
    active.role = Set(role);
    active.updated_at = Set(time::OffsetDateTime::now_utc());
    active.update(txn).await
}

pub async fn count_admins<C: ConnectionTrait + Send + Sync>(
    conn: &C,
) -> Result<u64, sea_orm::DbErr> {
    users::Entity::find()
        .filter(users::Column::Role.eq(users::UserRole::Admin))
        .count(conn)
        .await
}

pub async fn count_admins_excluding_user<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    exclude_user_id: i64,
) -> Result<u64, sea_orm::DbErr> {
    users::Entity::find()
        .filter(users::Column::Role.eq(users::UserRole::Admin))
        .filter(users::Column::Id.ne(exclude_user_id))
        .count(conn)
        .await
}

pub async fn create_user(
    txn: &DatabaseTransaction,
    dto: UserCreate,
) -> Result<users::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let user_active = users::ActiveModel {
        id: NotSet,
        username: Set(Some(dto.username)),
        is_ai: Set(dto.is_ai),
        role: Set(dto.role),
        created_at: Set(now),
        updated_at: Set(now),
    };

    user_active.insert(txn).await
}

pub async fn find_user_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    user_id: i64,
) -> Result<Option<users::Model>, sea_orm::DbErr> {
    users::Entity::find_by_id(user_id).one(conn).await
}

pub async fn delete_user(txn: &DatabaseTransaction, user_id: i64) -> Result<(), sea_orm::DbErr> {
    users::Entity::delete_by_id(user_id).exec(txn).await?;
    Ok(())
}
