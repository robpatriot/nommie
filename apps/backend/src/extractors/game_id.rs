use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpRequest};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

use crate::entities::games;
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::state::app_state::AppState;

/// Game ID extracted from the route path parameter
/// Validates that the game exists in the database
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameId(pub i64);

impl FromRequest for GameId {
    type Error = AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // Extract game_id from path parameters
            let game_id_str = req.match_info().get("game_id").ok_or_else(|| {
                AppError::bad_request(ErrorCode::InvalidGameId, "Missing game_id parameter")
            })?;

            // Parse to i64 and validate it's positive
            let game_id = game_id_str.parse::<i64>().map_err(|_| {
                AppError::bad_request(
                    ErrorCode::InvalidGameId,
                    format!("Invalid game id: {game_id_str}"),
                )
            })?;

            if game_id <= 0 {
                return Err(AppError::bad_request(
                    ErrorCode::InvalidGameId,
                    format!("Game id must be positive, got: {game_id}"),
                ));
            }

            // Get database connection from AppState
            let app_state = req
                .app_data::<web::Data<AppState>>()
                .ok_or_else(|| AppError::internal("AppState not available"))?;

            let db = app_state.db().ok_or_else(AppError::db_unavailable)?;

            // Check if game exists in database
            let game = games::Entity::find_by_id(game_id)
                .one(db)
                .await
                .map_err(|e| AppError::db(format!("Failed to query game: {e}")))?;

            let _game = game.ok_or_else(|| {
                AppError::not_found(
                    ErrorCode::GameNotFound,
                    format!("Game not found with id: {game_id}"),
                )
            })?;

            Ok(GameId(game_id))
        })
    }
}
