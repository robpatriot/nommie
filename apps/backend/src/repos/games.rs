//! Game repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::games_sea as games_adapter;
use crate::domain::state::Phase;
use crate::entities::games;
use crate::entities::games::GameState as DbGameState;
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

pub async fn create_game(
    txn: &DatabaseTransaction,
    dto: games_adapter::GameCreate,
) -> Result<Game, DomainError> {
    let game = games_adapter::create_game(txn, dto).await?;
    Ok(Game::from(game))
}

/// Convert database game state to domain phase.
///
/// This function maps the database representation (DbGameState) to the domain
/// representation (Phase). The database tracks implementation details like
/// Lobby and Dealing, while the domain focuses on game logic phases.
pub fn db_game_state_to_phase(db_state: &DbGameState, current_trick_no: i16) -> Phase {
    match *db_state {
        DbGameState::Lobby => Phase::Init,
        DbGameState::Dealing => Phase::Init,
        DbGameState::Bidding => Phase::Bidding,
        DbGameState::TrumpSelection => Phase::TrumpSelect,
        DbGameState::TrickPlay => Phase::Trick {
            trick_no: current_trick_no as u8,
        },
        DbGameState::Scoring => Phase::Scoring,
        DbGameState::BetweenRounds => Phase::Complete,
        DbGameState::Completed => Phase::GameOver,
        DbGameState::Abandoned => Phase::GameOver,
    }
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
