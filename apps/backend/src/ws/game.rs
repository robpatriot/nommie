use actix_web::web;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::db::txn::SharedTxn;
use crate::domain::snapshot::GameSnapshot;
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::extractors::current_user::CurrentUser;
use crate::protocol::game_state::ViewerState;
use crate::routes::games::{build_snapshot_response, build_snapshot_response_in_txn};
use crate::state::app_state::AppState;

/// Authorize a user's subscription to a game topic.
///
/// IMPORTANT: when `txn` is Some, we must query using that transaction so websocket
/// tests can see uncommitted rows created inside the per-test transaction.
pub async fn authorize_game_subscription(
    txn: Option<&SharedTxn>,
    app_state: &web::Data<AppState>,
    game_id: i64,
    user: &CurrentUser,
) -> Result<(), AppError> {
    use crate::entities::game_players;

    let membership = if let Some(shared) = txn {
        game_players::Entity::find()
            .filter(game_players::Column::GameId.eq(game_id))
            .filter(game_players::Column::HumanUserId.eq(Some(user.id)))
            .one(shared.transaction())
            .await?
    } else {
        let db = crate::db::require_db(app_state.as_ref())?;
        game_players::Entity::find()
            .filter(game_players::Column::GameId.eq(game_id))
            .filter(game_players::Column::HumanUserId.eq(Some(user.id)))
            .one(db)
            .await?
    };

    if membership.is_none() {
        return Err(AppError::Forbidden {
            code: ErrorCode::Forbidden,
            detail: "Not a member of this game".to_string(),
        });
    }

    Ok(())
}

/// Build the game state payload for the websocket protocol.
///
/// Returns:
/// - version: i32
/// - game: GameSnapshot (pure domain snapshot)
/// - viewer: ViewerState (viewer-relative context: seat/hand/constraints)
pub async fn build_game_state(
    txn: Option<&SharedTxn>,
    app_state: &web::Data<AppState>,
    game_id: i64,
    user: &CurrentUser,
) -> Result<(i32, GameSnapshot, ViewerState), AppError> {
    let (snapshot_response, _viewer_seat) = if let Some(shared) = txn {
        build_snapshot_response_in_txn(shared.transaction(), app_state, game_id, user.id).await?
    } else {
        // Inside websocket actors we do not have a request; passing None is fine.
        build_snapshot_response(None, app_state, game_id, user).await?
    };

    let version = snapshot_response.version;
    let game = snapshot_response.snapshot;
    let viewer = snapshot_response.viewer;

    Ok((version, game, viewer))
}
