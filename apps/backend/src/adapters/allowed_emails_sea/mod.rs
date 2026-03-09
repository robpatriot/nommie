//! SeaORM adapter for allowed_emails (admission table).

use sea_orm::sea_query::OnConflict;
use sea_orm::{ConnectionTrait, DatabaseTransaction, EntityTrait, NotSet, Set};

use crate::entities::allowed_emails;

pub async fn list_all<C: ConnectionTrait + Send + Sync>(
    conn: &C,
) -> Result<Vec<String>, sea_orm::DbErr> {
    let models = allowed_emails::Entity::find().all(conn).await?;
    Ok(models.into_iter().map(|m| m.email).collect())
}

pub async fn insert_if_not_exists(
    txn: &DatabaseTransaction,
    email: &str,
) -> Result<bool, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let active = allowed_emails::ActiveModel {
        id: NotSet,
        email: Set(email.to_string()),
        created_at: Set(now),
    };

    let result = allowed_emails::Entity::insert(active)
        .on_conflict(
            OnConflict::columns([allowed_emails::Column::Email])
                .do_nothing()
                .to_owned(),
        )
        .exec_without_returning(txn)
        .await?;

    Ok(result == 1)
}
