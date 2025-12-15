use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpMessage, HttpRequest};

use crate::auth::claims::BackendClaims;
use crate::db::require_db;
use crate::db::txn::SharedTxn;
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::repos::memberships::GameMembership as ServiceGameMembership;
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

async fn resolve_membership(req: HttpRequest) -> Result<GameMembership, AppError> {
    let claims = req
        .extensions()
        .get::<BackendClaims>()
        .ok_or_else(AppError::unauthorized_missing_bearer)?
        .clone();

    let game_id_value = req
        .match_info()
        .get("game_id")
        .ok_or_else(|| {
            AppError::bad_request(ErrorCode::InvalidGameId, "Missing game_id parameter")
        })?
        .parse::<i64>()
        .map_err(|_| AppError::bad_request(ErrorCode::InvalidGameId, "Invalid game id"))?;

    if game_id_value <= 0 {
        return Err(AppError::bad_request(
            ErrorCode::InvalidGameId,
            "Game id must be positive",
        ));
    }

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

    let user = if let Some(shared_txn) = SharedTxn::from_req(&req) {
        users::find_user_by_sub(shared_txn.transaction(), &claims.sub).await?
    } else {
        let db = require_db(app_state)?;
        users::find_user_by_sub(db, &claims.sub).await?
    };

    let user = user.ok_or_else(|| {
        AppError::forbidden_with_code(
            ErrorCode::ForbiddenUserNotFound,
            "User not found in database",
        )
    })?;

    let service = MembershipService;
    let membership = if let Some(shared_txn) = SharedTxn::from_req(&req) {
        service
            .find_membership(shared_txn.transaction(), game_id_value, user.id)
            .await?
    } else {
        let db = require_db(app_state)?;
        service.find_membership(db, game_id_value, user.id).await?
    };

    let membership = membership.ok_or_else(|| {
        AppError::forbidden_with_code(
            ErrorCode::NotAMember,
            format!("User is not a member of game {}", game_id_value),
        )
    })?;

    Ok(membership)
}

impl FromRequest for GameMembership {
    type Error = AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move { resolve_membership(req).await })
    }
}
