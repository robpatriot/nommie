//! DTOs for memberships_sea adapter.

/// DTO for updating a game membership.
#[derive(Debug, Clone)]
pub struct MembershipUpdate {
    pub id: i64,
    pub game_id: i64,
    pub player_kind: crate::entities::game_players::PlayerKind,
    pub human_user_id: Option<i64>,
    pub original_user_id: Option<i64>,
    pub ai_profile_id: Option<i64>,
    pub turn_order: Option<i16>,
    pub is_ready: bool,
}

/// DTO for creating a new game membership.
#[derive(Debug, Clone)]
pub struct MembershipCreate {
    pub game_id: i64,
    pub player_kind: crate::entities::game_players::PlayerKind,
    pub human_user_id: Option<i64>,
    pub original_user_id: Option<i64>,
    pub ai_profile_id: Option<i64>,
    pub turn_order: Option<i16>,
    pub is_ready: bool,
    pub role: crate::entities::game_players::GameRole,
}

/// DTO for setting a membership's ready status.
#[derive(Debug, Clone)]
pub struct MembershipSetReady {
    pub id: i64,
    pub is_ready: bool,
}

impl MembershipSetReady {
    pub fn new(id: i64, is_ready: bool) -> Self {
        Self { id, is_ready }
    }
}
