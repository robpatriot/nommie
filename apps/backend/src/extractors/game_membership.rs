use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpRequest};

use super::current_user::CurrentUser;
use super::game_id::GameId;
use crate::db::require_db;
use crate::db::txn::SharedTxn;
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::repos::memberships::{GameMembership as ServiceGameMembership, GameRole};
use crate::repos::users;
use crate::services::memberships::MembershipService;
use crate::state::app_state::AppState;

/// Game membership extractor that verifies a user's membership in a game
///
/// This extractor depends on CurrentUser and GameId extractors and provides
/// role-based access control for game operations.
///
/// # Example Usage
///
/// ```rust
/// async fn game_handler(
///     current_user: CurrentUser,
///     game_id: GameId,
///     membership: GameMembership,
/// ) -> Result<impl Responder, AppError> {
///     // User is guaranteed to be a member of the game
///     // Access membership details via membership.game_id, membership.user_id, etc.
///     Ok(web::Json(membership))
/// }
/// ```
/// Re-export the service GameMembership as the extractor GameMembership
pub type GameMembership = ServiceGameMembership;

/// Role guard for membership validation
#[derive(Debug, Clone, Copy)]
pub enum RoleGuard {
    /// Any member (no role restriction)
    Any,
    /// At least the specified role
    AtLeast(GameRole),
    /// Exactly the specified role
    Exactly(GameRole),
}

impl RoleGuard {
    /// Check if the membership satisfies this role guard
    pub fn check(&self, membership: &ServiceGameMembership) -> bool {
        match self {
            RoleGuard::Any => true,
            RoleGuard::AtLeast(required) => membership.role.has_at_least(*required),
            RoleGuard::Exactly(required) => membership.role == *required,
        }
    }
}

/// GameMembership extractor with role guard
pub struct GameMembershipWithGuard {
    pub membership: GameMembership,
    pub guard: RoleGuard,
}

impl FromRequest for GameMembership {
    type Error = AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();

        Box::pin(async move {
            // Extract CurrentUser and GameId first
            let current_user = CurrentUser::from_request(&req, &mut payload).await?;
            let game_id = GameId::from_request(&req, &mut payload).await?;

            // Get database connection from AppState
            let app_state = req.app_data::<web::Data<AppState>>().ok_or_else(|| {
                AppError::internal(
                    crate::errors::ErrorCode::InternalError,
                    "AppState not available".to_string(),
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "AppState missing from request",
                    ),
                )
            })?;

            // Find user by sub to get user_id
            let user = if let Some(shared_txn) = SharedTxn::from_req(&req) {
                users::find_user_by_sub(shared_txn.transaction(), &current_user.sub).await?
            } else {
                let db = require_db(app_state)?;
                users::find_user_by_sub(db, &current_user.sub).await?
            };

            let user = user.ok_or_else(|| {
                AppError::forbidden_with_code(
                    ErrorCode::ForbiddenUserNotFound,
                    "User not found in database",
                )
            })?;

            // Resolve repo from AppState and call service with correct conn
            let service = MembershipService;
            let membership = if let Some(shared_txn) = SharedTxn::from_req(&req) {
                service
                    .find_membership(shared_txn.transaction(), game_id.0, user.id)
                    .await?
            } else {
                let db = require_db(app_state)?;
                service.find_membership(db, game_id.0, user.id).await?
            };

            let membership = membership.ok_or_else(|| {
                AppError::forbidden_with_code(
                    ErrorCode::NotAMember,
                    format!("User is not a member of game {}", game_id.0),
                )
            })?;

            Ok(membership)
        })
    }
}

impl FromRequest for GameMembershipWithGuard {
    type Error = AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();

        Box::pin(async move {
            // Extract basic membership first
            let membership = GameMembership::from_request(&req, &mut payload).await?;

            // For now, we'll use Any role guard by default
            // In the future, this could be parameterized based on route or other factors
            let guard = RoleGuard::Any;

            if !guard.check(&membership) {
                return Err(AppError::forbidden_with_code(
                    ErrorCode::InsufficientRole,
                    "User does not have sufficient role for this operation",
                ));
            }

            Ok(GameMembershipWithGuard { membership, guard })
        })
    }
}
