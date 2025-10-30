use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpRequest};
use serde::{Deserialize, Serialize};

use crate::db::require_db;
use crate::db::txn::SharedTxn;
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::repos::games;
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

            // Check if game exists in database
            let game = if let Some(shared_txn) = SharedTxn::from_req(&req) {
                // Use shared transaction if present
                games::find_by_id(shared_txn.transaction(), game_id).await?
            } else {
                // Fall back to pooled connection
                let db = require_db(app_state)?;
                games::find_by_id(db, game_id).await?
            };

            game.ok_or_else(|| {
                AppError::not_found(
                    ErrorCode::GameNotFound,
                    format!("Game not found with id: {game_id}"),
                )
            })?;

            Ok(GameId(game_id))
        })
    }
}
