use sea_orm::ConnectionTrait;

use crate::errors::domain::DomainError;
use crate::repos::memberships::{self, GameMembership};

/// Membership domain service.
#[derive(Default)]
pub struct MembershipService;

impl MembershipService {
    /// Find a user's membership in a specific game
    pub async fn find_membership<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
        user_id: i64,
    ) -> Result<Option<GameMembership>, DomainError> {
        memberships::find_membership(conn, game_id, user_id).await
    }
}
