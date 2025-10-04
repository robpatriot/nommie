
use crate::db::DbConn;
use crate::error::AppError;
use crate::repos::memberships::{GameMembership, MembershipRepo};

/// Membership domain service.
pub struct MembershipService;

impl MembershipService {
    pub fn new() -> Self { Self }

    /// Find a user's membership in a specific game
    pub async fn find_membership(
        &self,
        repo: &dyn MembershipRepo,
        conn: &dyn DbConn,
        game_id: i64,
        user_id: i64,
    ) -> Result<Option<GameMembership>, AppError> {
        repo.find_membership(conn, game_id, user_id).await.map_err(AppError::from)
    }
}

impl Default for MembershipService {
    fn default() -> Self { Self::new() }
}

