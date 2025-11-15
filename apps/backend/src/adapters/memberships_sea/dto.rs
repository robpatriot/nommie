//! DTOs for memberships_sea adapter.

/// DTO for updating a game membership.
#[derive(Debug, Clone)]
pub struct MembershipUpdate {
    pub id: i64,
    pub game_id: i64,
    pub player_kind: crate::entities::game_players::PlayerKind,
    pub human_user_id: Option<i64>,
    pub ai_profile_id: Option<i64>,
    pub turn_order: i32,
    pub is_ready: bool,
}

impl MembershipUpdate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        game_id: i64,
        player_kind: crate::entities::game_players::PlayerKind,
        human_user_id: Option<i64>,
        ai_profile_id: Option<i64>,
        turn_order: i32,
        is_ready: bool,
    ) -> Self {
        Self {
            id,
            game_id,
            player_kind,
            human_user_id,
            ai_profile_id,
            turn_order,
            is_ready,
        }
    }
}

/// DTO for creating a new game membership.
#[derive(Debug, Clone)]
pub struct MembershipCreate {
    pub game_id: i64,
    pub player_kind: crate::entities::game_players::PlayerKind,
    pub human_user_id: Option<i64>,
    pub ai_profile_id: Option<i64>,
    pub turn_order: i32,
    pub is_ready: bool,
}

impl MembershipCreate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        game_id: i64,
        player_kind: crate::entities::game_players::PlayerKind,
        human_user_id: Option<i64>,
        ai_profile_id: Option<i64>,
        turn_order: i32,
        is_ready: bool,
    ) -> Self {
        Self {
            game_id,
            player_kind,
            human_user_id,
            ai_profile_id,
            turn_order,
            is_ready,
        }
    }
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
