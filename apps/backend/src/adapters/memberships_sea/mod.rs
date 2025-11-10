//! SeaORM adapter for membership repository.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, NotSet,
    QueryFilter, Set,
};

use crate::entities::game_players;

pub mod dto;

pub use dto::{MembershipCreate, MembershipSetReady, MembershipUpdate};

// Adapter functions return DbErr; repos layer maps to DomainError via From<DbErr>.

pub async fn find_membership<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
    user_id: i64,
) -> Result<Option<game_players::Model>, sea_orm::DbErr> {
    game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(game_id))
        .filter(game_players::Column::UserId.eq(user_id))
        .one(conn)
        .await
}

pub async fn create_membership(
    txn: &DatabaseTransaction,
    dto: MembershipCreate,
) -> Result<game_players::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let membership_active = game_players::ActiveModel {
        id: NotSet,
        game_id: Set(dto.game_id),
        user_id: Set(dto.user_id),
        turn_order: Set(dto.turn_order),
        is_ready: Set(dto.is_ready),
        created_at: Set(now),
        updated_at: Set(now),
    };

    membership_active.insert(txn).await
}

pub async fn update_membership(
    txn: &DatabaseTransaction,
    dto: MembershipUpdate,
) -> Result<game_players::Model, sea_orm::DbErr> {
    let membership = game_players::ActiveModel {
        id: Set(dto.id),
        game_id: Set(dto.game_id),
        user_id: Set(dto.user_id),
        turn_order: Set(dto.turn_order),
        is_ready: Set(dto.is_ready),
        created_at: NotSet,
        updated_at: Set(time::OffsetDateTime::now_utc()),
    };
    membership.update(txn).await
}

pub async fn set_membership_ready(
    txn: &DatabaseTransaction,
    dto: MembershipSetReady,
) -> Result<game_players::Model, sea_orm::DbErr> {
    let membership = game_players::ActiveModel {
        id: Set(dto.id),
        game_id: NotSet,
        user_id: NotSet,
        turn_order: NotSet,
        is_ready: Set(dto.is_ready),
        created_at: NotSet,
        updated_at: Set(time::OffsetDateTime::now_utc()),
    };
    membership.update(txn).await
}

pub async fn find_all_by_game<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
) -> Result<Vec<game_players::Model>, sea_orm::DbErr> {
    game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(game_id))
        .all(conn)
        .await
}

pub async fn delete_membership(txn: &DatabaseTransaction, id: i64) -> Result<(), sea_orm::DbErr> {
    game_players::Entity::delete_many()
        .filter(game_players::Column::Id.eq(id))
        .exec(txn)
        .await?;
    Ok(())
}
