//! Game-related HTTP routes.

use actix_web::{web, HttpRequest, HttpResponse, Result};

use crate::adapters::players_sea::PlayerRepoSea;
use crate::db::txn::with_txn;
use crate::error::AppError;
use crate::extractors::game_id::GameId;
use crate::services::games::load_game_state;
use crate::services::players::PlayerService;
use crate::state::app_state::AppState;

/// GET /api/games/{game_id}/snapshot
///
/// Returns the current game snapshot as JSON.
/// This is a read-only endpoint that produces a public view of the game state
/// without exposing private information (e.g., other players' hands).
async fn get_snapshot(
    http_req: HttpRequest,
    game_id: GameId,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;

    // Load game state and produce snapshot within a transaction
    let snapshot = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            // Load the game state (currently a stub; will load from DB once persistence is implemented)
            let state = load_game_state(id, txn).await?;

            // Produce the snapshot via the domain function
            let snap = crate::domain::snapshot::snapshot(&state);

            Ok(snap)
        })
    })
    .await?;

    Ok(HttpResponse::Ok().json(snapshot))
}

/// GET /api/games/{game_id}/players/{seat}/display_name
///
/// Returns the display name of the player at the specified seat in the game.
async fn get_player_display_name(
    http_req: HttpRequest,
    path: web::Path<(i64, u8)>,
    app_state: web::Data<AppState>,
) -> Result<web::Json<PlayerDisplayNameResponse>, AppError> {
    let (game_id, seat) = path.into_inner();

    // Get display name within a transaction
    let display_name = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            // Create SeaORM adapter with the transaction
            let repo = PlayerRepoSea::new(txn);
            let service = PlayerService::new(repo);
            
            service.get_display_name_by_seat(game_id, seat).await
        })
    })
    .await?;

    // Return JSON response
    Ok(web::Json(PlayerDisplayNameResponse { display_name }))
}

#[derive(serde::Serialize)]
struct PlayerDisplayNameResponse {
    display_name: String,
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/api/games/{game_id}/snapshot").route(web::get().to(get_snapshot)));
    cfg.service(web::resource("/api/games/{game_id}/players/{seat}/display_name").route(web::get().to(get_player_display_name)));
}
