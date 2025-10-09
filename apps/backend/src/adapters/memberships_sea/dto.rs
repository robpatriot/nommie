//! DTOs for memberships_sea adapter.

/// DTO for creating a new game membership.
#[derive(Debug, Clone)]
pub struct MembershipCreate {
    pub game_id: i64,
    pub user_id: i64,
    pub turn_order: i32,
    pub is_ready: bool,
}

impl MembershipCreate {
    pub fn new(game_id: i64, user_id: i64, turn_order: i32, is_ready: bool) -> Self {
        Self {
            game_id,
            user_id,
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
