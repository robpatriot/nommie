//! Game repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::games_sea as games_adapter;
use crate::entities::games;
use crate::errors::domain::DomainError;

/// Game domain model
#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub id: i64,
    pub join_code: String,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

// Free functions (generic) mirroring the previous trait methods

pub async fn find_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
) -> Result<Option<Game>, DomainError> {
    let game = games_adapter::find_by_id(conn, game_id).await?;
    Ok(game.map(Game::from))
}

pub async fn find_by_join_code<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    join_code: &str,
) -> Result<Option<Game>, DomainError> {
    let game = games_adapter::find_by_join_code(conn, join_code).await?;
    Ok(game.map(Game::from))
}

pub async fn create_game(txn: &DatabaseTransaction, join_code: &str) -> Result<Game, DomainError> {
    let dto = games_adapter::GameCreate::new(join_code);
    let game = games_adapter::create_game(txn, dto).await?;
    Ok(Game::from(game))
}

// Conversions between SeaORM models and domain models

impl From<games::Model> for Game {
    fn from(model: games::Model) -> Self {
        Self {
            id: model.id,
            join_code: model.join_code.unwrap_or_default(),
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
