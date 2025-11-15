//! SeaORM adapter for player repository - generic over ConnectionTrait.

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::entities::game_players;

// Adapter functions return DbErr; repos layer maps to DomainError via From<DbErr>.

pub async fn get_player_by_seat<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
    seat: u8,
) -> Result<Option<game_players::Model>, sea_orm::DbErr> {
    game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(game_id))
        .filter(game_players::Column::TurnOrder.eq(seat as i32))
        .one(conn)
        .await
}
