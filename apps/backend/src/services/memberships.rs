use crate::db::DbConn;
use crate::errors::domain::DomainError;
use crate::repos::memberships::{GameMembership, MembershipRepo};

/// Membership domain service.
pub struct MembershipService;

impl MembershipService {
    pub fn new() -> Self {
        Self
    }

    /// Find a user's membership in a specific game
    pub async fn find_membership(
        &self,
        repo: &dyn MembershipRepo,
        conn: &dyn DbConn,
        game_id: i64,
        user_id: i64,
    ) -> Result<Option<GameMembership>, DomainError> {
        repo.find_membership(conn, game_id, user_id).await
    }
}

impl Default for MembershipService {
    fn default() -> Self {
        Self::new()
    }
}
