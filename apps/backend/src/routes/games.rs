//! Game-related HTTP routes.

use actix_web::http::header::{ETAG, IF_NONE_MATCH};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, Result};

use crate::adapters::games_sea;
use crate::db::txn::with_txn;
use crate::error::AppError;
use crate::extractors::game_id::GameId;
use crate::http::etag::game_etag;
use crate::services::players::PlayerService;
use crate::state::app_state::AppState;

/// GET /api/games/{game_id}/snapshot
///
/// Returns the current game snapshot as JSON with an ETag header for optimistic concurrency.
/// This is a read-only endpoint that produces a public view of the game state
/// without exposing private information (e.g., other players' hands).
///
/// Supports `If-None-Match` for HTTP caching: if the client's ETag matches the current version,
/// returns `304 Not Modified` with no body.
async fn get_snapshot(
    http_req: HttpRequest,
    game_id: GameId,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;

    // Load game state and produce snapshot within a transaction
    let (snapshot, lock_version) = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            // Fetch game from database to get lock_version
            let game = games_sea::find_by_id(txn, id)
                .await
                .map_err(|e| AppError::db(format!("Failed to fetch game: {e}")))?
                .ok_or_else(|| {
                    AppError::not_found(
                        crate::errors::ErrorCode::GameNotFound,
                        format!("Game with ID {id} not found"),
                    )
                })?;

            // Create game service and load game state
            let game_service = crate::services::games::GameService::new();
            let state = game_service.load_game_state(id).await?;

            // Produce the snapshot via the domain function
            let snap = crate::domain::snapshot::snapshot(&state);

            Ok((snap, game.lock_version))
        })
    })
    .await?;

    // Generate ETag from game ID and lock version
    let etag_value = game_etag(id, lock_version);

    // Check If-None-Match header for HTTP caching
    if let Some(if_none_match) = http_req.headers().get(IF_NONE_MATCH) {
        if let Ok(client_etag) = if_none_match.to_str() {
            // Check for wildcard match (RFC 9110) or specific ETag match
            // Wildcard "*" means "any representation exists"
            let matches = client_etag.trim() == "*"
                || client_etag
                    .split(',')
                    .map(str::trim)
                    .any(|etag| etag == etag_value);

            if matches {
                // Resource hasn't changed, return 304 Not Modified
                return Ok(HttpResponse::build(StatusCode::NOT_MODIFIED)
                    .insert_header((ETAG, etag_value))
                    .finish());
            }
        }
    }

    // Resource is new or modified, return full response
    Ok(HttpResponse::Ok()
        .insert_header((ETAG, etag_value))
        .json(snapshot))
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
            let service = PlayerService::new();
            Ok(service.get_display_name_by_seat(txn, game_id, seat).await?)
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
    cfg.service(
        web::resource("/api/games/{game_id}/players/{seat}/display_name")
            .route(web::get().to(get_player_display_name)),
    );
}
