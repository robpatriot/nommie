//! SeaORM adapter for player repository - generic over ConnectionTrait.

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::entities::{game_players, users};

// Adapter functions return DbErr; repos layer maps to DomainError via From<DbErr>.

pub async fn get_display_name_by_seat<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
    seat: u8,
) -> Result<Option<(game_players::Model, users::Model)>, sea_orm::DbErr> {
    let result = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(game_id))
        .filter(game_players::Column::TurnOrder.eq(seat as i32))
        .find_also_related(users::Entity)
        .one(conn)
        .await?;

    match result {
        Some((gp, Some(user))) => Ok(Some((gp, user))),
        Some((_gp, None)) => Ok(None), // User not found for game player
        None => Ok(None),              // No game player found
    }
}
