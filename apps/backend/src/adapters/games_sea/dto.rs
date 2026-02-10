//! DTOs for games_sea adapter.

use time::OffsetDateTime;

use crate::entities::games::{GameState, GameVisibility};

/// DTO for creating a new game.
#[derive(Debug, Clone, Default)]
pub struct GameCreate {
    pub created_by: Option<i64>,
    pub visibility: Option<GameVisibility>,
    pub name: Option<String>,
}

impl GameCreate {
    pub fn new() -> Self {
        Self {
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

/// Unified DTO for updating game fields with optimistic locking.
///
/// Can update any combination of state, round-related fields (current_round, starting_dealer_pos, current_trick_no).
/// All updates are atomic with a single version increment.
///
/// `expected_version` validates that the current version matches before updating.
#[derive(Debug, Clone)]
pub struct GameUpdate {
    pub id: i64,
    pub state: Option<GameState>,
    pub current_round: Option<u8>,
    pub starting_dealer_pos: Option<u8>,
    pub current_trick_no: Option<u8>,
    /// Three-state: None = no change, Some(Some(ts)) = set, Some(None) = clear.
    pub waiting_since: Option<Option<OffsetDateTime>>,
    /// Three-state: None = no change, Some(Some(ts)) = set, Some(None) = clear.
    pub started_at: Option<Option<OffsetDateTime>>,
    /// Three-state: None = no change, Some(Some(ts)) = set, Some(None) = clear.
    pub ended_at: Option<Option<OffsetDateTime>>,
    pub expected_version: i32,
}

impl GameUpdate {
    pub fn new(id: i64, expected_version: i32) -> Self {
        Self {
            id,
            state: None,
            current_round: None,
            starting_dealer_pos: None,
            current_trick_no: None,
            waiting_since: None,
            started_at: None,
            ended_at: None,
            expected_version,
        }
    }

    pub fn with_state(mut self, state: GameState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn with_current_round(mut self, round: u8) -> Self {
        self.current_round = Some(round);
        self
    }

    pub fn with_starting_dealer_pos(mut self, pos: u8) -> Self {
        self.starting_dealer_pos = Some(pos);
        self
    }

    pub fn with_current_trick_no(mut self, trick_no: u8) -> Self {
        self.current_trick_no = Some(trick_no);
        self
    }

    pub fn with_waiting_since(mut self, waiting_since: OffsetDateTime) -> Self {
        self.waiting_since = Some(Some(waiting_since));
        self
    }

    pub fn clear_waiting_since(mut self) -> Self {
        self.waiting_since = Some(None);
        self
    }

    pub fn with_started_at(mut self, started_at: OffsetDateTime) -> Self {
        self.started_at = Some(Some(started_at));
        self
    }

    pub fn clear_started_at(mut self) -> Self {
        self.started_at = Some(None);
        self
    }

    pub fn with_ended_at(mut self, ended_at: OffsetDateTime) -> Self {
        self.ended_at = Some(Some(ended_at));
        self
    }

    pub fn clear_ended_at(mut self) -> Self {
        self.ended_at = Some(None);
        self
    }
}
