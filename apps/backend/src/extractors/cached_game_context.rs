//! Cached game context extractor for HTTP requests.
//!
//! This extractor loads and caches a complete GameContext (game_id, history, round_info)
//! for the authenticated user's game participation. The context is cached in request
//! extensions to avoid redundant database queries within a single request.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpMessage, HttpRequest};

use super::game_id::GameId;
use super::game_membership::GameMembership;
use crate::db::require_db;
use crate::db::txn::SharedTxn;
use crate::domain::game_context::GameContext;
use crate::error::AppError;
use crate::repos::{games, player_view};
use crate::state::app_state::AppState;

/// Request-scoped cached game context.
///
/// This extractor provides a complete view of the game state including:
/// - Game ID
/// - Game history (if game has started)
/// - Current round info for the authenticated player (if in active round)
///
/// The context is cached in request extensions, so multiple consumers
/// within the same request (handler, services, nested calls) get the
/// same instance without additional database queries.
///
/// # Trust Boundary
///
/// This context is for **UI rendering and response building only**.
/// Services should NOT accept GameContext for validation - they load their
/// own data to maintain proper trust boundaries. Services are security
/// boundaries and must not rely on caller-provided data for security checks.
///
/// # Example
///
/// ```rust,ignore
/// async fn submit_bid(
///     context: CachedGameContext,
///     body: ValidatedJson<BidRequest>,
/// ) -> Result<HttpResponse, AppError> {
///     let ctx = context.context();
///     
///     // Use context for UI rendering
///     let game_id = ctx.game_id;
///     let history = ctx.game_history();
///     
///     // Service loads its own validation data (trust boundary)
///     game_flow_service.submit_bid(txn, game_id, player_seat, bid_value).await?;
///     
///     Ok(HttpResponse::Ok().finish())
/// }
/// ```
#[derive(Clone)]
pub struct CachedGameContext(pub Arc<GameContext>);

impl CachedGameContext {
    /// Get a reference to the underlying game context.
    pub fn context(&self) -> &GameContext {
        &self.0
    }

    /// Extract a CachedGameContext from request extensions if already loaded.
    fn from_extensions(req: &HttpRequest) -> Option<Self> {
        req.extensions().get::<Self>().cloned()
    }

    /// Insert this cached context into request extensions.
    fn insert_into_extensions(&self, req: &HttpRequest) {
        req.extensions_mut().insert(self.clone());
    }
}

impl FromRequest for CachedGameContext {
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();

        Box::pin(async move {
            // Check if already cached in request extensions
            if let Some(cached) = Self::from_extensions(&req) {
                return Ok(cached);
            }

            // Extract dependencies
            let game_id = GameId::from_request(&req, &mut payload).await?;
            let membership = GameMembership::from_request(&req, &mut payload).await?;
            let player_seat = membership.turn_order as i16;

            // Get database connection and load data
            let app_state = req
                .app_data::<actix_web::web::Data<AppState>>()
                .ok_or_else(|| {
                    AppError::internal(
                        crate::errors::ErrorCode::InternalError,
                        "AppState not available".to_string(),
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "AppState missing from request",
                        ),
                    )
                })?;

            // Start building context
            let mut context = GameContext::new(game_id.0);

            // Load data using either shared transaction or pooled connection
            if let Some(shared_txn) = SharedTxn::from_req(&req) {
                // Use shared transaction
                let txn = shared_txn.transaction();

                // Load game to check if it has started
                let game = games::require_game(txn, game_id.0).await?;

                // If game has started, load history and round info
                if game.current_round.is_some() {
                    let history = player_view::load_game_history(txn, game_id.0).await?;
                    context = context.with_history(history);

                    let round_info =
                        player_view::load_current_round_info(txn, game_id.0, player_seat).await?;
                    context = context.with_round_info(round_info);
                }
            } else {
                // Use pooled connection
                let db = require_db(app_state)?;

                // Load game to check if it has started
                let game = games::require_game(db, game_id.0).await?;

                // If game has started, load history and round info
                if game.current_round.is_some() {
                    let history = player_view::load_game_history(db, game_id.0).await?;
                    context = context.with_history(history);

                    let round_info =
                        player_view::load_current_round_info(db, game_id.0, player_seat).await?;
                    context = context.with_round_info(round_info);
                }
            }

            // Cache and return
            let cached = CachedGameContext(Arc::new(context));
            cached.insert_into_extensions(&req);

            Ok(cached)
        })
    }
}
