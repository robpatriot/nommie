//! SeaORM adapter for membership repository - generic over ConnectionTrait.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, NotSet, QueryFilter, Set,
};

use crate::entities::game_players;

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

pub async fn create_membership<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
    user_id: i64,
    turn_order: i32,
    is_ready: bool,
) -> Result<game_players::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let membership_active = game_players::ActiveModel {
        id: NotSet,
        game_id: Set(game_id),
        user_id: Set(user_id),
        turn_order: Set(turn_order),
        is_ready: Set(is_ready),
        created_at: Set(now),
    };

    membership_active.insert(conn).await
}

pub async fn update_membership<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    id: i64,
    game_id: i64,
    user_id: i64,
    turn_order: i32,
    is_ready: bool,
) -> Result<game_players::Model, sea_orm::DbErr> {
    // Note: game_players table doesn't have updated_at field
    let membership = game_players::ActiveModel {
        id: Set(id),
        game_id: Set(game_id),
        user_id: Set(user_id),
        turn_order: Set(turn_order),
        is_ready: Set(is_ready),
        created_at: Set(time::OffsetDateTime::now_utc()),
    };
    membership.update(conn).await
}
