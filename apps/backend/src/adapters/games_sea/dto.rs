//! DTOs for games_sea adapter.

use crate::entities::games::{GameState, GameVisibility};

/// DTO for creating a new game.
#[derive(Debug, Clone)]
pub struct GameCreate {
    pub join_code: String,
    pub created_by: Option<i64>,
    pub visibility: Option<GameVisibility>,
    pub name: Option<String>,
}

impl GameCreate {
    pub fn new(
        join_code: impl Into<String>,
        created_by: Option<i64>,
        visibility: Option<GameVisibility>,
        name: Option<impl Into<String>>,
    ) -> Self {
        Self {
            join_code: join_code.into(),
            created_by,
            visibility,
            name: name.map(|n| n.into()),
        }
    }
}

/// DTO for updating game state.
#[derive(Debug, Clone)]
pub struct GameUpdateState {
    pub id: i64,
    pub state: GameState,
}

impl GameUpdateState {
    pub fn new(id: i64, state: GameState) -> Self {
        Self { id, state }
    }
}

/// DTO for updating game metadata.
#[derive(Debug, Clone)]
pub struct GameUpdateMetadata {
    pub id: i64,
    pub name: Option<String>,
    pub visibility: GameVisibility,
}

impl GameUpdateMetadata {
    pub fn new(id: i64, name: Option<impl Into<String>>, visibility: GameVisibility) -> Self {
        Self {
            id,
            name: name.map(|n| n.into()),
            visibility,
        }
    }
}

/// DTO for updating game round data.
#[derive(Debug, Clone)]
pub struct GameUpdateRound {
    pub id: i64,
    pub current_round: Option<i16>,
    pub hand_size: Option<i16>,
    pub dealer_pos: Option<i16>,
}

impl GameUpdateRound {
    pub fn new(
        id: i64,
        current_round: Option<i16>,
        hand_size: Option<i16>,
        dealer_pos: Option<i16>,
    ) -> Self {
        Self {
            id,
            current_round,
            hand_size,
            dealer_pos,
        }
    }
}
