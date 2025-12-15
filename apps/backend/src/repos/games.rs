//! Game repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::games_sea as games_adapter;
use crate::domain::state::Phase;
use crate::entities::games;
use crate::entities::games::GameState as DbGameState;
use crate::errors::domain::DomainError;

/// Game domain model
///
/// This represents a game in the domain layer, with all fields needed for
/// game logic and state management. It's converted from the database model
/// (games::Model) when loaded through repos functions.
#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub id: i64,
    pub created_by: Option<i64>,
    pub visibility: games::GameVisibility,
    pub state: DbGameState,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
    pub started_at: Option<time::OffsetDateTime>,
    pub ended_at: Option<time::OffsetDateTime>,
    pub name: Option<String>,
    pub join_code: Option<String>,
    pub rules_version: String,
    pub rng_seed: Option<i64>,
    pub current_round: Option<u8>,
    pub starting_dealer_pos: Option<u8>,
    pub current_trick_no: u8,
    pub current_round_id: Option<i64>,
    pub lock_version: i32,
}

impl Game {
    /// Computes the current hand size based on the current round number.
    /// Returns None if current_round is None or out of valid range.
    pub fn hand_size(&self) -> Option<u8> {
        use crate::domain::rules;

        let round_no = self.current_round?;
        if !(1..=26).contains(&round_no) {
            return None;
        }

        rules::hand_size_for_round(round_no)
    }

    /// Computes the current dealer position based on starting dealer and current round.
    /// Returns None if either starting_dealer_pos or current_round is None.
    pub fn dealer_pos(&self) -> Option<u8> {
        let starting = self.starting_dealer_pos?;
        let round = self.current_round?;

        // Dealer rotates each round: (starting_dealer + round_no - 1) % 4
        // Subtract 1 because round_no starts at 1, not 0
        Some((starting + (round - 1)) % 4)
    }
}

// Free functions (generic) mirroring the previous trait methods

pub async fn find_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
) -> Result<Option<Game>, DomainError> {
    let game = games_adapter::find_by_id(conn, game_id).await?;
    Ok(game.map(Game::from))
}

/// Find game by join code (test-only helper)
#[allow(dead_code)] // Used in adapter tests and test helpers
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

/// Find game by ID or return error if not found.
///
/// This is a convenience helper that converts `None` into a DomainError,
/// eliminating the repetitive `ok_or_else` pattern when a game must exist.
pub async fn require_game<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
) -> Result<Game, DomainError> {
    let game = games_adapter::require_game(conn, game_id).await?;
    Ok(Game::from(game))
}

/// Update game with optimistic locking.
///
/// Updates any combination of state and round-related fields (current_round, starting_dealer_pos, current_trick_no)
/// atomically with a single lock_version increment. Returns the updated game model.
///
/// `expected_lock_version` validates that the current lock_version matches before updating.
pub async fn update_game(
    txn: &DatabaseTransaction,
    id: i64,
    expected_lock_version: i32,
    state: Option<DbGameState>,
    current_round: Option<u8>,
    starting_dealer_pos: Option<u8>,
    current_trick_no: Option<u8>,
) -> Result<Game, DomainError> {
    let mut dto = games_adapter::GameUpdate::new(id, expected_lock_version);
    if let Some(s) = state {
        dto = dto.with_state(s);
    }
    if let Some(round) = current_round {
        dto = dto.with_current_round(round);
    }
    if let Some(pos) = starting_dealer_pos {
        dto = dto.with_starting_dealer_pos(pos);
    }
    if let Some(trick_no) = current_trick_no {
        dto = dto.with_current_trick_no(trick_no);
    }
    let game = games_adapter::update_game(txn, dto).await?;
    Ok(Game::from(game))
}

/// Touch game to increment lock_version without changing any game fields.
///
/// This is useful when membership or other related data changes that affect the game snapshot
/// but don't require updating any fields on the games table itself. Increments lock_version
/// and updates updated_at to trigger websocket broadcasts.
///
/// `expected_lock_version` validates that the current lock_version matches before updating.
pub async fn touch_game(
    txn: &DatabaseTransaction,
    id: i64,
    expected_lock_version: i32,
) -> Result<Game, DomainError> {
    update_game(txn, id, expected_lock_version, None, None, None, None).await
}

/// Delete game with optimistic locking.
///
/// `expected_lock_version` validates that the current lock_version matches before deleting.
pub async fn delete_game(
    txn: &DatabaseTransaction,
    id: i64,
    expected_lock_version: i32,
) -> Result<(), DomainError> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    use crate::entities::games;

    let delete_result = games::Entity::delete_many()
        .filter(games::Column::Id.eq(id))
        .filter(games::Column::LockVersion.eq(expected_lock_version))
        .exec(txn)
        .await?;

    if delete_result.rows_affected == 0 {
        // Check if game exists to distinguish between NotFound and OptimisticLock
        let game = games_adapter::find_by_id(txn, id).await?;
        if let Some(game) = game {
            // Lock version mismatch
            return Err(DomainError::conflict(
                crate::errors::domain::ConflictKind::OptimisticLock,
                format!(
                    "Game lock version mismatch: expected {}, but game has version {}",
                    expected_lock_version, game.lock_version
                ),
            ));
        }
        // Game doesn't exist - that's fine for delete (idempotent)
    }

    Ok(())
}

/// Convert database game state to domain phase.
///
/// This function maps the database representation (DbGameState) to the domain
/// representation (Phase). The database tracks implementation details like
/// Lobby and Dealing, while the domain focuses on game logic phases.
pub fn db_game_state_to_phase(db_state: &DbGameState, current_trick_no: u8) -> Phase {
    match *db_state {
        DbGameState::Lobby => Phase::Init,
        DbGameState::Dealing => Phase::Init,
        DbGameState::Bidding => Phase::Bidding,
        DbGameState::TrumpSelection => Phase::TrumpSelect,
        DbGameState::TrickPlay => Phase::Trick {
            trick_no: current_trick_no,
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
            created_by: model.created_by,
            visibility: model.visibility,
            state: model.state,
            created_at: model.created_at,
            updated_at: model.updated_at,
            started_at: model.started_at,
            ended_at: model.ended_at,
            name: model.name,
            join_code: model.join_code,
            rules_version: model.rules_version,
            rng_seed: model.rng_seed,
            current_round: model.current_round.map(|v| v as u8),
            starting_dealer_pos: model.starting_dealer_pos.map(|v| v as u8),
            current_trick_no: model.current_trick_no as u8,
            current_round_id: model.current_round_id,
            lock_version: model.lock_version,
        }
    }
}
