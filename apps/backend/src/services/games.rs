//! Game state loading and construction services.

use crate::db::DbConn;
use crate::domain::fixtures::CardFixtures;
use crate::domain::state::{GameState, Phase, RoundState};
use crate::errors::domain::{DomainError, ValidationKind};

/// Game domain service.
pub struct GameService;

impl GameService {
    pub fn new() -> Self {
        Self
    }

    /// Load or construct a GameState for the given game_id.
    ///
    /// Currently returns a stub state since full game state persistence is not yet implemented.
    /// This will be replaced with actual DB loading once state serialization is added.
    pub async fn load_game_state(&self, game_id: i64) -> Result<GameState, DomainError> {
        // For now, return a deterministic stub state for testing
        // TODO: Load actual game state from database once persistence is implemented

        if game_id <= 0 {
            return Err(DomainError::validation(
                ValidationKind::InvalidGameId,
                "Game ID must be positive",
            ));
        }

        // Build a minimal bidding-phase state for demonstration
        let hands = [
            CardFixtures::parse_hardcoded(&["AC", "2C", "3C", "4C", "5C"]),
            CardFixtures::parse_hardcoded(&["AD", "2D", "3D", "4D", "5D"]),
            CardFixtures::parse_hardcoded(&["AH", "2H", "3H", "4H", "5H"]),
            CardFixtures::parse_hardcoded(&["AS", "2S", "3S", "4S", "5S"]),
        ];

        Ok(GameState {
            phase: Phase::Bidding,
            round_no: 1,
            hand_size: 5,
            hands,
            turn_start: 1,
            turn: 1,
            leader: 1,
            trick_no: 0,
            scores_total: [0, 0, 0, 0],
            round: RoundState::new(),
        })
    }
}

impl Default for GameService {
    fn default() -> Self {
        Self::new()
    }
}

/// Legacy function for backward compatibility
/// TODO: Remove once all callers are updated to use GameService
pub async fn load_game_state(game_id: i64, _conn: &dyn DbConn) -> Result<GameState, DomainError> {
    if game_id <= 0 {
        return Err(DomainError::validation(
            ValidationKind::InvalidGameId,
            "Game ID must be positive",
        ));
    }

    // Build a minimal bidding-phase state for demonstration
    let hands = [
        CardFixtures::parse_hardcoded(&["AC", "2C", "3C", "4C", "5C"]),
        CardFixtures::parse_hardcoded(&["AD", "2D", "3D", "4D", "5D"]),
        CardFixtures::parse_hardcoded(&["AH", "2H", "3H", "4H", "5H"]),
        CardFixtures::parse_hardcoded(&["AS", "2S", "3S", "4S", "5S"]),
    ];

    Ok(GameState {
        phase: Phase::Bidding,
        round_no: 1,
        hand_size: 5,
        hands,
        turn_start: 1,
        turn: 1,
        leader: 1,
        trick_no: 0,
        scores_total: [0, 0, 0, 0],
        round: RoundState::new(),
    })
}
