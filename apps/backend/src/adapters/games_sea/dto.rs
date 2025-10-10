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
    pub fn new(join_code: impl Into<String>) -> Self {
        Self {
            join_code: join_code.into(),
            created_by: None,
            visibility: None,
            name: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_visibility(mut self, visibility: GameVisibility) -> Self {
        self.visibility = Some(visibility);
        self
    }

    pub fn by(mut self, user_id: i64) -> Self {
        self.created_by = Some(user_id);
        self
    }
}

/// DTO for updating game state.
#[derive(Debug, Clone)]
pub struct GameUpdateState {
    pub id: i64,
    pub state: GameState,
    pub current_lock_version: i32,
}

impl GameUpdateState {
    pub fn new(id: i64, state: GameState, current_lock_version: i32) -> Self {
        Self {
            id,
            state,
            current_lock_version,
        }
    }
}

/// DTO for updating game metadata.
#[derive(Debug, Clone)]
pub struct GameUpdateMetadata {
    pub id: i64,
    pub name: Option<String>,
    pub visibility: GameVisibility,
    pub current_lock_version: i32,
}

impl GameUpdateMetadata {
    pub fn new(
        id: i64,
        name: Option<impl Into<String>>,
        visibility: GameVisibility,
        current_lock_version: i32,
    ) -> Self {
        Self {
            id,
            name: name.map(|n| n.into()),
            visibility,
            current_lock_version,
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
    pub current_lock_version: i32,
}

impl GameUpdateRound {
    pub fn new(id: i64, current_lock_version: i32) -> Self {
        Self {
            id,
            current_round: None,
            hand_size: None,
            dealer_pos: None,
            current_lock_version,
        }
    }

    pub fn with_current_round(mut self, round: i16) -> Self {
        self.current_round = Some(round);
        self
    }

    pub fn with_hand_size(mut self, size: i16) -> Self {
        self.hand_size = Some(size);
        self
    }

    pub fn with_dealer_pos(mut self, pos: i16) -> Self {
        self.dealer_pos = Some(pos);
        self
    }
}
