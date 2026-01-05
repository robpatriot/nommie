//! Game-related HTTP routes.

use std::collections::HashSet;

use actix_web::http::header::{ETAG, IF_NONE_MATCH};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use rand::random;
use sea_orm::TransactionTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::format_description::well_known::Rfc3339;
use tracing::{debug, info, warn};

use crate::ai::{registry, RandomPlayer, Strategic};
use crate::db::txn::with_txn;
use crate::domain::bidding::validate_consecutive_zero_bids;
use crate::domain::snapshot::{GameSnapshot, SeatAiProfilePublic, SeatPublic};
use crate::domain::state::Seat;
use crate::domain::{Card, Rank, Suit, Trump};
use crate::entities::games::{GameState, GameVisibility};
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::extractors::current_user::CurrentUser;
use crate::extractors::game_id::GameId;
use crate::extractors::game_membership::GameMembership;
use crate::extractors::ValidatedJson;
use crate::http::etag::game_etag;
use crate::repos::memberships::{self, GameRole};
use crate::repos::players::{friendly_ai_name, resolve_display_name_for_membership};
use crate::repos::{ai_overrides, ai_profiles, games as games_repo, player_view, users};
use crate::routes::snapshot_cache::SharedSnapshotParts;
use crate::services::ai::{AiInstanceOverrides, AiService};
use crate::services::game_flow::GameFlowService;
use crate::services::games::GameService;
use crate::services::players::PlayerService;
use crate::state::app_state::AppState;

#[derive(Clone, Serialize)]
pub(crate) struct GameSnapshotResponse {
    snapshot: GameSnapshot,
    viewer_hand: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bid_constraints: Option<BidConstraintsResponse>,
    pub(crate) version: i32,
}

#[derive(Clone, Serialize)]
pub(crate) struct BidConstraintsResponse {
    zero_bid_locked: bool,
}

/// Join a user to a game and touch the game to trigger WebSocket broadcasts.
///
/// This helper encapsulates the common pattern of joining a user to a game
/// and incrementing the game's version so that WebSocket clients receive
/// updates about the new player.
///
/// Returns the updated game and all memberships.
async fn join_game_and_touch(
    txn: &sea_orm::DatabaseTransaction,
    game_id: i64,
    user_id: i64,
    initial_version: i32,
) -> Result<(games_repo::Game, Vec<memberships::GameMembership>), AppError> {
    let service = GameService;
    let (_game, memberships) = service.join_game(txn, game_id, user_id).await?;

    // Touch game to increment version so WebSocket clients receive the update
    let final_game = games_repo::touch_game(txn, game_id, initial_version).await?;

    Ok((final_game, memberships))
}

/// Helper that leaves a game and touches it to increment version.
///
/// This helper encapsulates the common pattern of removing a user from a game
/// and incrementing the game's version so that WebSocket clients receive
/// updates about the player leaving.
///
/// For active games, converts the human to AI, touches the game (incrementing version),
/// then processes game state (AIs may act, each incrementing version further).
async fn leave_game_and_touch(
    txn: &sea_orm::DatabaseTransaction,
    game_id: i64,
    user_id: i64,
    initial_version: i32,
) -> Result<i32, AppError> {
    // Check game state before leaving to determine if we need AI processing
    let game_before = games_repo::require_game(txn, game_id).await?;
    let was_active = game_before.state != crate::entities::games::GameState::Lobby;

    let service = GameService;
    service.leave_game(txn, game_id, user_id).await?;

    // If game was active, the human was converted to AI
    if was_active {
        // Touch game to increment version for the conversion (unit of work)
        let updated_game = games_repo::touch_game(txn, game_id, initial_version).await?;
        // Note: AI processing is handled in a background task after transaction commits
        Ok(updated_game.version)
    } else {
        // Game was in Lobby - just touch to increment version
        let updated_game = games_repo::touch_game(txn, game_id, initial_version).await?;
        Ok(updated_game.version)
    }
}

/// Build shared snapshot parts (cacheable by game_id + version).
///
/// This is the expensive part that loads game state, memberships, and builds
/// the public snapshot and seating array. The result is cached to avoid redundant
/// work when multiple users receive broadcasts for the same game version.
async fn build_shared_snapshot_parts(
    txn: &sea_orm::DatabaseTransaction,
    game_id: i64,
) -> Result<SharedSnapshotParts, AppError> {
    // Fetch game from database to get version
    let game = games_repo::require_game(txn, game_id).await?;

    // Create game service and load game state
    let game_service = crate::services::games::GameService;
    let state = game_service.load_game_state(txn, game_id).await?;

    let memberships = memberships::find_all_by_game(txn, game_id)
        .await
        .map_err(AppError::from)?;

    // Produce the snapshot via the domain function
    let mut snap = crate::domain::snapshot::snapshot(&state);

    let mut seating = [
        SeatPublic::empty(0),
        SeatPublic::empty(1),
        SeatPublic::empty(2),
        SeatPublic::empty(3),
    ];

    let mut host_seat: Seat = 0;
    let creator_user_id = game.created_by;

    for membership in memberships {
        let Some(seat_idx) = membership.turn_order else {
            // Spectators don't have a turn_order, skip them
            continue;
        };
        if !(0..4).contains(&seat_idx) {
            continue;
        }

        let seat = seat_idx;
        let mut seat_public = SeatPublic::empty(seat);
        seat_public.is_ready = membership.is_ready;

        // Use consolidated function to resolve display name (with final fallback)
        let display_name = resolve_display_name_for_membership(txn, &membership, seat, true)
            .await
            .map_err(AppError::from)?;

        seat_public.display_name = Some(display_name);

        // Set original_user_id if present (for rejoin detection)
        seat_public.original_user_id = membership.original_user_id;

        // Build additional SeatPublic fields
        match membership.player_kind {
            crate::entities::game_players::PlayerKind::Human => {
                if let Some(user_id) = membership.user_id {
                    seat_public.user_id = Some(user_id);
                    let user = users::find_user_by_id(txn, user_id)
                        .await
                        .map_err(AppError::from)?
                        .ok_or_else(|| {
                            AppError::not_found(
                                ErrorCode::UserNotFound,
                                format!("User {user_id} not found"),
                            )
                        })?;

                    seat_public.is_ai = user.is_ai;

                    if creator_user_id == Some(user_id) {
                        host_seat = seat;
                    }
                }
            }
            crate::entities::game_players::PlayerKind::Ai => {
                seat_public.is_ai = true;
                if let Some(profile_id) = membership.ai_profile_id {
                    let profile = ai_profiles::find_by_id(txn, profile_id)
                        .await
                        .map_err(AppError::from)?;

                    if let Some(profile) = profile.as_ref() {
                        seat_public.ai_profile = Some(SeatAiProfilePublic {
                            name: profile.registry_name.clone(),
                            version: profile.registry_version.clone(),
                        });
                    }
                }
            }
        }

        seating[seat_idx as usize] = seat_public;
    }

    snap.game.seating = seating.clone();
    snap.game.host_seat = host_seat;

    let version = game.version;
    Ok(SharedSnapshotParts {
        game,
        snapshot: snap,
        seating,
        version,
    })
}

/// Build user-specific snapshot parts (not cacheable, per-user).
///
/// Determines viewer_seat, loads viewer_hand, and computes bid_constraints
/// for a specific user. This is called after getting shared parts from cache.
async fn build_user_specific_parts(
    txn: &sea_orm::DatabaseTransaction,
    game_id: i64,
    current_user_id: i64,
    shared: &SharedSnapshotParts,
) -> Result<
    (
        Option<Seat>,
        Option<Vec<String>>,
        Option<BidConstraintsResponse>,
    ),
    AppError,
> {
    // Determine viewer_seat from memberships (check if current_user_id matches any seat)
    let mut viewer_seat: Option<Seat> = None;
    for (seat_idx, seat_public) in shared.seating.iter().enumerate() {
        if let Some(user_id) = seat_public.user_id {
            if user_id == current_user_id {
                viewer_seat = Some(seat_idx as Seat);
                break;
            }
        }
    }

    // Load viewer hand if viewer is a player
    // Only load if game has started (current_round is Some and > 0)
    let viewer_hand = if let Some(seat) = viewer_seat {
        if shared.game.current_round.is_some() {
            match player_view::load_current_round_info(txn, game_id, seat).await {
                Ok(info) => Some(info.hand.into_iter().map(format_card).collect::<Vec<_>>()),
                Err(err) => {
                    warn!(game_id = game_id, seat, error = %err, "Failed to load viewer hand");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Compute bid_constraints if in bidding phase
    let mut bid_constraints: Option<BidConstraintsResponse> = None;
    if matches!(
        &shared.snapshot.phase,
        crate::domain::snapshot::PhaseSnapshot::Bidding(_)
    ) {
        if let Some(current_round_no) = shared.game.current_round {
            if let Some(viewer_seat_value) = viewer_seat {
                let history = player_view::load_game_history(txn, game_id).await?;
                let zero_bid_locked =
                    validate_consecutive_zero_bids(&history, viewer_seat_value, current_round_no)
                        .is_err();
                bid_constraints = Some(BidConstraintsResponse { zero_bid_locked });
            }
        }
    }

    Ok((viewer_seat, viewer_hand, bid_constraints))
}

pub(crate) async fn build_snapshot_response(
    http_req: Option<&HttpRequest>,
    app_state: &web::Data<AppState>,
    game_id: i64,
    current_user: &CurrentUser,
) -> Result<(GameSnapshotResponse, Option<Seat>), AppError> {
    let current_user_id = current_user.id;
    let cache = app_state.snapshot_cache_arc();

    // Get shared parts from cache (with deduplication)
    // We need to get the version first to form the cache key, then get or build
    let shared = with_txn(http_req, app_state, |txn| {
        let cache = cache.clone();
        Box::pin(async move {
            // Get current version first to form cache key
            let game = games_repo::require_game(txn, game_id).await?;
            let cache_key = (game_id, game.version);

            // Check cache first
            if let Some(cached) = cache.get(cache_key) {
                return Ok(cached);
            }

            // Cache miss - build shared parts
            let shared_parts = build_shared_snapshot_parts(txn, game_id).await?;

            // Insert into cache with deduplication (get_or_insert handles double-check)
            Ok(cache.get_or_insert(cache_key, shared_parts).await)
        })
    })
    .await?;

    // Clone shared before moving into closure
    let shared_clone = shared.clone();

    // Build user-specific parts
    let (viewer_seat, viewer_hand, bid_constraints) = with_txn(http_req, app_state, |txn| {
        Box::pin(async move {
            build_user_specific_parts(txn, game_id, current_user_id, &shared_clone).await
        })
    })
    .await?;

    let response = GameSnapshotResponse {
        snapshot: shared.snapshot.clone(),
        viewer_hand,
        bid_constraints,
        version: shared.version,
    };

    Ok((response, viewer_seat))
}

pub(crate) async fn build_snapshot_response_in_txn(
    txn: &sea_orm::DatabaseTransaction,
    app_state: &AppState,
    game_id: i64,
    current_user_id: i64,
) -> Result<(GameSnapshotResponse, Option<Seat>), AppError> {
    let cache = app_state.snapshot_cache();

    // Get current version to form cache key
    let game = games_repo::require_game(txn, game_id).await?;
    let cache_key = (game_id, game.version);

    // Check cache first
    let shared = if let Some(cached) = cache.get(cache_key) {
        cached
    } else {
        // Cache miss - build shared parts
        let shared_parts = build_shared_snapshot_parts(txn, game_id).await?;

        // Insert into cache with deduplication (get_or_insert handles double-check)
        cache.get_or_insert(cache_key, shared_parts).await
    };

    // Build user-specific parts
    let (viewer_seat, viewer_hand, bid_constraints) =
        build_user_specific_parts(txn, game_id, current_user_id, &shared).await?;

    // Note: We don't publish broadcasts here since we're inside a transaction.
    // The caller should handle broadcasting after the transaction commits if needed.

    let response = GameSnapshotResponse {
        snapshot: shared.snapshot.clone(),
        viewer_hand,
        bid_constraints,
        version: shared.version,
    };

    Ok((response, viewer_seat))
}

#[derive(Serialize)]
struct GameHistoryResponse {
    rounds: Vec<RoundHistoryResponse>,
}

#[derive(Serialize)]
struct RoundHistoryResponse {
    round_no: u8,
    hand_size: u8,
    dealer_seat: u8,
    bids: [Option<u8>; 4],
    trump_selector_seat: Option<u8>,
    trump: Option<&'static str>,
    cumulative_scores: [i16; 4],
}

#[derive(Serialize)]
struct AiRegistryEntryResponse {
    name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct AiRegistryListResponse {
    ais: Vec<AiRegistryEntryResponse>,
}

async fn list_registered_ais() -> Result<web::Json<AiRegistryListResponse>, AppError> {
    let ais = registry::registered_ais()
        .iter()
        .map(|factory| AiRegistryEntryResponse {
            name: factory.name,
            version: factory.version,
        })
        .collect();

    Ok(web::Json(AiRegistryListResponse { ais }))
}

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
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;

    // Load game state and produce snapshot within a transaction
    let (snapshot_response, viewer_seat) =
        build_snapshot_response(Some(&http_req), &app_state, id, &current_user).await?;

    // Generate ETag from game ID and lock version
    let etag_value = game_etag(id, snapshot_response.version);

    // Check If-None-Match header for HTTP caching
    if let Some(if_none_match) = http_req.headers().get(IF_NONE_MATCH) {
        debug!(game_id = id, "Found If-None-Match header present");
        if let Ok(client_etag) = if_none_match.to_str() {
            debug!(game_id = id, client_etag = %client_etag, current_etag = %etag_value, "Received If-None-Match ETag header");
            // Check for wildcard match (RFC 9110) or specific ETag match
            // Wildcard "*" means "any representation exists"
            let matches = client_etag.trim() == "*"
                || client_etag
                    .split(',')
                    .map(str::trim)
                    .any(|etag| etag == etag_value);

            if matches {
                debug!(game_id = id, "ETag match found, returning 304 Not Modified");
                // Resource hasn't changed, return 304 Not Modified
                let mut not_modified = HttpResponse::build(StatusCode::NOT_MODIFIED);
                not_modified.insert_header((ETAG, etag_value));
                if let Some(seat) = viewer_seat {
                    not_modified.insert_header(("x-viewer-seat", seat.to_string()));
                }
                return Ok(not_modified.finish());
            } else {
                debug!(game_id = id, "ETag mismatch, returning full response");
            }
        } else {
            debug!(
                game_id = id,
                "Failed to convert If-None-Match header to string"
            );
        }
    }

    // Resource is new or modified, return full response
    let mut response = HttpResponse::Ok();
    response.insert_header((ETAG, etag_value));
    if let Some(seat) = viewer_seat {
        response.insert_header(("x-viewer-seat", seat.to_string()));
    }
    Ok(response.json(snapshot_response))
}

/// GET /api/games/{game_id}/history
///
/// Returns the game history with an ETag header for HTTP caching.
/// Supports `If-None-Match` for HTTP caching: if the client's ETag matches the current version,
/// returns `304 Not Modified` with no body.
async fn get_game_history(
    http_req: HttpRequest,
    game_id: GameId,
    _membership: GameMembership,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;

    // Load game to get version and history within a transaction
    let (history, version) = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            // Fetch game from database to get version
            let game = games_repo::require_game(txn, id).await?;

            // Load game history
            let history = player_view::load_game_history(txn, id).await?;

            Ok((history, game.version))
        })
    })
    .await?;

    // Generate ETag from game ID and lock version
    let etag_value = game_etag(id, version);

    // Check If-None-Match header for HTTP caching
    if let Some(if_none_match) = http_req.headers().get(IF_NONE_MATCH) {
        debug!(game_id = id, "Found If-None-Match header present");
        if let Ok(client_etag) = if_none_match.to_str() {
            debug!(
                game_id = id,
                client_etag = %client_etag,
                current_etag = %etag_value,
                "Received If-None-Match ETag header"
            );
            // Check for wildcard match (RFC 9110) or specific ETag match
            let matches = client_etag.trim() == "*"
                || client_etag
                    .split(',')
                    .map(str::trim)
                    .any(|etag| etag == etag_value);

            if matches {
                info!(game_id = id, "ETag cache hit, returning 304 Not Modified");
                // Resource hasn't changed, return 304 Not Modified
                let mut not_modified = HttpResponse::build(StatusCode::NOT_MODIFIED);
                not_modified.insert_header((ETAG, etag_value));
                return Ok(not_modified.finish());
            } else {
                debug!(game_id = id, "ETag mismatch, returning full response");
            }
        } else {
            debug!(
                game_id = id,
                "Failed to convert If-None-Match header to string"
            );
        }
    }

    let rounds = history
        .rounds
        .into_iter()
        .map(|round| RoundHistoryResponse {
            round_no: round.round_no,
            hand_size: round.hand_size,
            dealer_seat: round.dealer_seat,
            bids: round.bids,
            trump_selector_seat: round.trump_selector_seat,
            trump: round.trump.map(trump_to_api_value),
            cumulative_scores: round
                .scores
                .map(|score_detail| score_detail.cumulative_score),
        })
        .collect();

    // Resource is new or modified, return full response
    let mut response = HttpResponse::Ok();
    response.insert_header((ETAG, etag_value));
    Ok(response.json(GameHistoryResponse { rounds }))
}

/// GET /api/games/{game_id}/players/{seat}/display_name
///
/// Returns the display name of the player at the specified seat in the game.
/// Supports `If-None-Match` for HTTP caching: if the client's ETag matches the current version,
/// returns `304 Not Modified` with no body.
async fn get_player_display_name(
    http_req: HttpRequest,
    path: web::Path<(i64, u8)>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let (game_id, seat) = path.into_inner();

    // Get display name and game version within a transaction
    let (display_name, version) = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            // Fetch game from database to get version
            let game = games_repo::require_game(txn, game_id).await?;

            // Get display name
            let service = PlayerService;
            let display_name = service.get_display_name_by_seat(txn, game_id, seat).await?;

            Ok((display_name, game.version))
        })
    })
    .await?;

    // Generate ETag from game ID and lock version
    let etag_value = game_etag(game_id, version);

    // Check If-None-Match header for HTTP caching
    if let Some(if_none_match) = http_req.headers().get(IF_NONE_MATCH) {
        debug!(game_id = game_id, "Found If-None-Match header present");
        if let Ok(client_etag) = if_none_match.to_str() {
            debug!(
                game_id = game_id,
                client_etag = %client_etag,
                current_etag = %etag_value,
                "Received If-None-Match ETag header"
            );
            // Check for wildcard match (RFC 9110) or specific ETag match
            let matches = client_etag.trim() == "*"
                || client_etag
                    .split(',')
                    .map(str::trim)
                    .any(|etag| etag == etag_value);

            if matches {
                info!(
                    game_id = game_id,
                    "ETag cache hit, returning 304 Not Modified"
                );
                // Resource hasn't changed, return 304 Not Modified
                let mut not_modified = HttpResponse::build(StatusCode::NOT_MODIFIED);
                not_modified.insert_header((ETAG, etag_value));
                return Ok(not_modified.finish());
            } else {
                debug!(game_id = game_id, "ETag mismatch, returning full response");
            }
        } else {
            debug!(
                game_id = game_id,
                "Failed to convert If-None-Match header to string"
            );
        }
    }

    // Resource is new or modified, return full response
    let mut response = HttpResponse::Ok();
    response.insert_header((ETAG, etag_value));
    Ok(response.json(PlayerDisplayNameResponse { display_name }))
}

#[derive(serde::Serialize)]
struct PlayerDisplayNameResponse {
    display_name: String,
}

// Request/Response DTOs for create and join endpoints

#[derive(serde::Deserialize)]
struct CreateGameRequest {
    name: Option<String>,
}

#[derive(serde::Serialize)]
struct CreateGameResponse {
    game: GameResponse,
}

#[derive(serde::Serialize)]
struct JoinGameResponse {
    game: GameResponse,
}

#[derive(serde::Serialize)]
struct GameListResponse {
    games: Vec<GameResponse>,
}

#[derive(serde::Serialize)]
struct LastActiveGameResponse {
    game_id: Option<i64>,
}

#[derive(serde::Serialize)]
struct GameResponse {
    id: i64,
    name: String,
    state: String,
    visibility: String,
    created_by: i64,
    created_at: String,
    updated_at: String,
    started_at: Option<String>,
    ended_at: Option<String>,
    current_round: Option<u8>,
    player_count: i32,
    max_players: i32,
    viewer_is_member: bool,
    viewer_is_host: bool,
    can_rejoin: bool,
}

/// Helper function to convert GameState enum to frontend string format
fn game_state_to_string(state: &GameState) -> String {
    match state {
        GameState::Lobby => "LOBBY".to_string(),
        GameState::Dealing => "DEALING".to_string(),
        GameState::Bidding => "BIDDING".to_string(),
        GameState::TrumpSelection => "TRUMP_SELECTION".to_string(),
        GameState::TrickPlay => "TRICK_PLAY".to_string(),
        GameState::Scoring => "SCORING".to_string(),
        GameState::BetweenRounds => "BETWEEN_ROUNDS".to_string(),
        GameState::Completed => "COMPLETED".to_string(),
        GameState::Abandoned => "ABANDONED".to_string(),
    }
}

/// Helper function to convert GameVisibility enum to frontend string format
fn game_visibility_to_string(visibility: &GameVisibility) -> String {
    match visibility {
        GameVisibility::Public => "PUBLIC".to_string(),
        GameVisibility::Private => "PRIVATE".to_string(),
    }
}

/// Helper function to format OffsetDateTime as RFC 3339 string
fn format_rfc3339(dt: &time::OffsetDateTime) -> String {
    dt.format(&Rfc3339).unwrap_or_else(|_| dt.to_string())
}

/// Check if a viewer is a human member of the game.
///
/// Returns true if the viewer has a human player membership in the game.
/// This excludes AI players and spectators.
fn is_human_member(
    viewer_user_id: Option<i64>,
    memberships: &[memberships::GameMembership],
) -> bool {
    viewer_user_id
        .and_then(|id| {
            memberships.iter().find(|m| {
                m.user_id == Some(id)
                    && m.role == GameRole::Player
                    && m.player_kind == crate::entities::game_players::PlayerKind::Human
            })
        })
        .is_some()
}

/// Helper function to convert game domain model + memberships to frontend Game format
///
/// Returns an error if the game does not have a creator (created_by is None).
/// This enforces that all games must have a creator (hard breaking change).
fn game_to_response(
    game: &crate::repos::games::Game,
    memberships: &[memberships::GameMembership],
    viewer_user_id: Option<i64>,
) -> Result<GameResponse, AppError> {
    let created_by = game.created_by.ok_or_else(|| {
        AppError::internal(
            ErrorCode::DataCorruption,
            format!(
                "Game {} does not have a creator (created_by is None)",
                game.id
            ),
            std::io::Error::other("game.created_by must be set"),
        )
    })?;

    // Count players (exclude spectators)
    let player_count = memberships
        .iter()
        .filter(|m| m.role == GameRole::Player)
        .count() as i32;

    // Check if viewer is a member (player or spectator)
    let viewer_is_member = is_human_member(viewer_user_id, memberships)
        || viewer_user_id
            .map(|id| {
                memberships.iter().any(|m| {
                    m.user_id == Some(id)
                        && m.role == GameRole::Spectator
                        && m.player_kind == crate::entities::game_players::PlayerKind::Human
                })
            })
            .unwrap_or(false);

    // Check if viewer is the host (creator of the game)
    let viewer_is_host = viewer_user_id.map(|id| created_by == id).unwrap_or(false);

    // Check if viewer can rejoin (has an AI seat with original_user_id matching viewer)
    let can_rejoin = viewer_user_id
        .map(|id| {
            memberships.iter().any(|m| {
                m.player_kind == crate::entities::game_players::PlayerKind::Ai
                    && m.original_user_id == Some(id)
            })
        })
        .unwrap_or(false);

    Ok(GameResponse {
        id: game.id,
        name: game
            .name
            .clone()
            .unwrap_or_else(|| format!("Game {}", game.id)),
        state: game_state_to_string(&game.state),
        visibility: game_visibility_to_string(&game.visibility),
        created_by,
        created_at: format_rfc3339(&game.created_at),
        updated_at: format_rfc3339(&game.updated_at),
        started_at: game.started_at.as_ref().map(format_rfc3339),
        ended_at: game.ended_at.as_ref().map(format_rfc3339),
        current_round: game.current_round,
        player_count,
        max_players: 4,
        viewer_is_member,
        viewer_is_host,
        can_rejoin,
    })
}

/// POST /api/games
///
/// Creates a new game and adds the creator as the first member.
async fn create_game(
    http_req: HttpRequest,
    current_user: CurrentUser,
    body: ValidatedJson<CreateGameRequest>,
    app_state: web::Data<AppState>,
) -> Result<web::Json<CreateGameResponse>, AppError> {
    let user_id = current_user.id;
    let game_name = body.name.clone();

    // Validate name length if provided (HTTP-level validation)
    if let Some(ref name) = game_name {
        if name.len() > 255 {
            return Err(AppError::bad_request(
                ErrorCode::ValidationError,
                "Game name must be 255 characters or less".to_string(),
            ));
        }
    }

    // Create game using service layer (handles join code generation, game creation, membership)
    let (game_model, memberships) = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameService;
            service
                .create_game_with_creator(txn, user_id, game_name)
                .await
        })
    })
    .await?;

    let response = game_to_response(&game_model, &memberships, Some(user_id))?;

    Ok(web::Json(CreateGameResponse { game: response }))
}

/// POST /api/games/{gameId}/join
///
/// Adds the current user as a member of the specified game.
async fn join_game(
    http_req: HttpRequest,
    game_id: GameId,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
) -> Result<web::Json<JoinGameResponse>, AppError> {
    let user_id = current_user.id;
    let id = game_id.0;

    // Join game using service layer (handles validation, seat assignment, membership creation)
    let (game_model, memberships) = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            // Fetch game to get initial version
            let game = games_repo::require_game(txn, id).await?;
            let initial_version = game.version;

            join_game_and_touch(txn, id, user_id, initial_version).await
        })
    })
    .await?;

    // Publish snapshot broadcast so other players' UIs update
    publish_snapshot(&app_state, id).await?;

    let response = game_to_response(&game_model, &memberships, Some(user_id))?;

    Ok(web::Json(JoinGameResponse { game: response }))
}

/// POST /api/games/{game_id}/spectate
///
/// Adds the current user as a spectator of the specified game.
/// Only public games can be spectated.
async fn spectate_game(
    http_req: HttpRequest,
    game_id: GameId,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
) -> Result<web::Json<JoinGameResponse>, AppError> {
    let user_id = current_user.id;
    let id = game_id.0;

    // Join as spectator using service layer (handles validation, visibility check, membership creation)
    let (game_model, memberships) = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            // Fetch game to get initial version
            let game = games_repo::require_game(txn, id).await?;
            let initial_version = game.version;

            let service = GameService;
            let (game, memberships) = service.join_as_spectator(txn, id, user_id).await?;

            // Touch game to trigger WebSocket broadcasts
            games_repo::touch_game(txn, id, initial_version).await?;

            Ok((game, memberships))
        })
    })
    .await?;

    // Publish snapshot broadcast so other viewers' UIs update
    publish_snapshot(&app_state, id).await?;

    let response = game_to_response(&game_model, &memberships, Some(user_id))?;

    Ok(web::Json(JoinGameResponse { game: response }))
}

/// DELETE /api/games/{gameId}/leave
///
/// Removes the current user from the specified game.
///
/// Requires version in request body for optimistic locking.
async fn leave_game(
    http_req: HttpRequest,
    game_id: GameId,
    current_user: CurrentUser,
    body: ValidatedJson<LeaveGameRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let user_id = current_user.id;
    let id = game_id.0;
    let expected_version = body.version;

    // Check if game is active before leaving (needed to determine if background task is needed)
    let db = crate::db::require_db(&app_state)?;
    let game_before = games_repo::require_game(db, id).await?;
    let was_active = game_before.state != crate::entities::games::GameState::Lobby;

    // Leave game using service layer
    let updated_version = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move { leave_game_and_touch(txn, id, user_id, expected_version).await })
    })
    .await?;

    let etag = game_etag(id, updated_version);

    // Publish snapshot broadcast so other players' UIs update
    publish_snapshot(&app_state, id).await?;

    // If game was active, spawn background task to process AI actions
    if was_active {
        // Clone AppState for background task (web::Data is Arc internally)
        let app_state_clone = app_state.clone();
        let game_id_for_task = id;

        tokio::spawn(async move {
            // Use a fresh database connection for the background task
            let db = match crate::db::require_db(&app_state_clone) {
                Ok(db) => db,
                Err(err) => {
                    tracing::error!(
                        game_id = game_id_for_task,
                        error = %err,
                        "Failed to get database connection for background AI processing"
                    );
                    return;
                }
            };

            // Begin a new transaction for AI processing
            let txn = match db.begin().await {
                Ok(txn) => txn,
                Err(err) => {
                    tracing::error!(
                        game_id = game_id_for_task,
                        error = %err,
                        "Failed to begin transaction for background AI processing"
                    );
                    return;
                }
            };

            // Process game state - AIs may act, each incrementing version
            let game_flow_service = crate::services::game_flow::GameFlowService;
            match game_flow_service
                .process_game_state(&txn, game_id_for_task)
                .await
            {
                Ok(()) => {
                    // Commit the transaction
                    if let Err(err) = txn.commit().await {
                        tracing::error!(
                            game_id = game_id_for_task,
                            error = %err,
                            "Failed to commit transaction after background AI processing"
                        );
                    } else {
                        tracing::debug!(
                            game_id = game_id_for_task,
                            "Background AI processing completed successfully"
                        );

                        // Publish snapshot broadcast so clients receive the AI action updates
                        if let Err(err) = publish_snapshot(&app_state_clone, game_id_for_task).await
                        {
                            tracing::error!(
                                game_id = game_id_for_task,
                                error = %err,
                                "Failed to publish snapshot after background AI processing"
                            );
                        }
                    }
                }
                Err(err) => {
                    tracing::error!(
                        game_id = game_id_for_task,
                        error = %err,
                        "Background AI processing failed"
                    );
                    // Best-effort rollback
                    let _ = txn.rollback().await;
                }
            }
        });
    }

    Ok(HttpResponse::NoContent()
        .insert_header((ETAG, etag))
        .finish())
}

/// GET /api/games/joinable
///
/// Returns a list of public lobby games that still have open seats.
async fn list_joinable_games(
    http_req: HttpRequest,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
) -> Result<web::Json<GameListResponse>, AppError> {
    let viewer_id = current_user.id;

    let games = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameService;
            service.list_joinable_games(txn).await
        })
    })
    .await?;

    let response_games = games
        .into_iter()
        .map(|(game_model, memberships)| {
            game_to_response(&game_model, &memberships, Some(viewer_id))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(web::Json(GameListResponse {
        games: response_games,
    }))
}

/// GET /api/games/in-progress
///
/// Returns a list of games that are currently active (non-lobby, non-finished).
async fn list_in_progress_games(
    http_req: HttpRequest,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
) -> Result<web::Json<GameListResponse>, AppError> {
    let viewer_id = current_user.id;

    let games = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameService;
            service.list_active_games(txn).await
        })
    })
    .await?;

    let mut visible_games = Vec::new();

    for (game_model, memberships) in games {
        let viewer_is_member = is_human_member(Some(viewer_id), &memberships);
        // Check if viewer is a spectator
        let viewer_is_spectator = memberships.iter().any(|m| {
            m.user_id == Some(viewer_id)
                && m.role == GameRole::Spectator
                && m.player_kind == crate::entities::game_players::PlayerKind::Human
        });

        if game_model.visibility == GameVisibility::Public
            || viewer_is_member
            || viewer_is_spectator
        {
            visible_games.push(game_to_response(
                &game_model,
                &memberships,
                Some(viewer_id),
            )?);
        }
    }

    Ok(web::Json(GameListResponse {
        games: visible_games,
    }))
}

/// GET /api/games/overview
///
/// Returns a combined list of lobby and in-progress games the viewer can see.
async fn list_overview_games(
    http_req: HttpRequest,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
) -> Result<web::Json<GameListResponse>, AppError> {
    let viewer_id = current_user.id;

    let lobby_games = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameService;
            service.list_public_lobby_games(txn).await
        })
    })
    .await?;

    let active_games = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameService;
            service.list_active_games(txn).await
        })
    })
    .await?;

    let mut combined_games = Vec::new();

    for (game_model, memberships) in lobby_games {
        combined_games.push(game_to_response(
            &game_model,
            &memberships,
            Some(viewer_id),
        )?);
    }

    for (game_model, memberships) in active_games {
        let viewer_is_member = is_human_member(Some(viewer_id), &memberships);
        // Check if viewer is a spectator
        let viewer_is_spectator = memberships.iter().any(|m| {
            m.user_id == Some(viewer_id)
                && m.role == GameRole::Spectator
                && m.player_kind == crate::entities::game_players::PlayerKind::Human
        });
        if game_model.visibility == GameVisibility::Public
            || viewer_is_member
            || viewer_is_spectator
        {
            combined_games.push(game_to_response(
                &game_model,
                &memberships,
                Some(viewer_id),
            )?);
        }
    }

    combined_games.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(web::Json(GameListResponse {
        games: combined_games,
    }))
}

/// GET /api/games/last-active
///
/// Returns the game ID of the game that has been waiting for the user to act the longest.
/// If no games are waiting for the user, returns the most recently active game
/// (highest updated_at timestamp) where the user is a member.
async fn get_last_active_game(
    http_req: HttpRequest,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
) -> Result<web::Json<LastActiveGameResponse>, AppError> {
    let user_id = current_user.id;

    let game_id = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameService;
            service.find_last_active_game(txn, user_id).await
        })
    })
    .await?;

    Ok(web::Json(LastActiveGameResponse { game_id }))
}

#[derive(serde::Deserialize)]
struct MarkReadyRequest {
    is_ready: bool,
}

/// POST /api/games/{game_id}/ready
///
/// Sets the ready status of the current player. When all four seats are ready, the game
/// automatically transitions into the first round.
async fn mark_ready(
    http_req: HttpRequest,
    game_id: GameId,
    current_user: CurrentUser,
    body: ValidatedJson<MarkReadyRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let user_id = current_user.id;
    let is_ready = body.is_ready;

    let updated_version = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameFlowService;
            service.set_ready_status(txn, id, user_id, is_ready).await?;
            // Fetch game to get updated version for ETag
            let game = games_repo::require_game(txn, id).await?;
            Ok(game.version)
        })
    })
    .await?;

    let etag = game_etag(id, updated_version);

    // Publish snapshot broadcast so other players' UIs update
    publish_snapshot(&app_state, id).await?;

    Ok(HttpResponse::NoContent()
        .insert_header((ETAG, etag))
        .finish())
}

#[derive(serde::Deserialize)]
struct SubmitBidRequest {
    bid: u8,
    version: i32,
}

#[derive(serde::Deserialize)]
struct SetTrumpRequest {
    trump: String,
    version: i32,
}

#[derive(serde::Deserialize)]
struct PlayCardRequest {
    card: String,
    version: i32,
}

#[derive(serde::Deserialize)]
struct LeaveGameRequest {
    version: i32,
}

#[derive(serde::Deserialize)]
struct RejoinGameRequest {
    version: i32,
}

#[derive(serde::Deserialize)]
struct DeleteGameRequest {
    version: i32,
}

#[derive(Debug, Deserialize)]
struct ManageAiSeatRequest {
    seat: Option<u8>,
    registry_name: Option<String>,
    registry_version: Option<String>,
    seed: Option<u64>,
    version: i32,
}

fn resolve_registry_selection(
    request: &ManageAiSeatRequest,
    default_name: Option<&'static str>,
) -> Result<(&'static crate::ai::registry::AiFactory, String, Option<u64>), AppError> {
    let name = match (request.registry_name.as_deref(), default_name) {
        (Some(name), _) => name,
        (None, Some(default)) => default,
        (None, None) => {
            return Err(AppError::bad_request(
                ErrorCode::ValidationError,
                "registry_name is required".to_string(),
            ));
        }
    };

    let factory = registry::by_name(name).ok_or_else(|| {
        AppError::bad_request(
            ErrorCode::ValidationError,
            format!("Unknown AI registry entry '{name}'"),
        )
    })?;

    if let Some(provided_version) = request.registry_version.as_deref() {
        if provided_version != factory.version {
            return Err(AppError::bad_request(
                ErrorCode::ValidationError,
                format!(
                    "Registry version '{}' does not match server version '{}' for '{}'",
                    provided_version, factory.version, factory.name
                ),
            ));
        }
    }

    let version = request
        .registry_version
        .clone()
        .unwrap_or_else(|| factory.version.to_string());

    Ok((factory, version, request.seed))
}

async fn add_ai_seat(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: ValidatedJson<ManageAiSeatRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let request = body.into_inner();
    let requested_seat = request.seat;
    let host_user_id = membership.user_id;
    // Note: version from request is not used - we fetch current game state
    // after add_ai_to_game which may have started the game and updated version
    let _version = request.version;

    let updated_version = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let game = games_repo::require_game(txn, id).await?;

            let game_service = GameService;
            if !game_service.is_host(&game, host_user_id) {
                return Err(AppError::forbidden_with_code(
                    ErrorCode::Forbidden,
                    "Only the host can manage AI seats",
                ));
            }

            if game.state != GameState::Lobby {
                return Err(AppError::bad_request(
                    ErrorCode::PhaseMismatch,
                    "AI seats can only be managed before the game starts",
                ));
            }

            let existing_memberships = memberships::find_all_by_game(txn, id)
                .await
                .map_err(AppError::from)?;

            // Count only players (exclude spectators)
            let player_count = existing_memberships
                .iter()
                .filter(|m| m.role == GameRole::Player)
                .count();

            if player_count >= 4 {
                return Err(AppError::conflict(
                    ErrorCode::SeatTaken,
                    "All seats are already filled",
                ));
            }

            let seat_to_fill = if let Some(seat_val) = requested_seat {
                if seat_val > 3 {
                    return Err(AppError::bad_request(
                        ErrorCode::InvalidSeat,
                        format!("Seat {seat_val} is out of range (0-3)"),
                    ));
                }
                if existing_memberships
                    .iter()
                    .any(|m| m.turn_order == Some(seat_val))
                {
                    return Err(AppError::conflict(
                        ErrorCode::SeatTaken,
                        format!("Seat {seat_val} is already taken"),
                    ));
                }
                seat_val
            } else {
                let game_service = GameService;
                // Filter to only players for seat finding
                let player_memberships: Vec<_> = existing_memberships
                    .iter()
                    .filter(|m| m.role == GameRole::Player)
                    .cloned()
                    .collect();
                game_service
                    .find_next_available_seat(&player_memberships)
                    .ok_or_else(|| {
                        AppError::conflict(ErrorCode::SeatTaken, "No available seats remaining")
                    })?
            };

            let seat_index = seat_to_fill;
            let mut existing_display_names: HashSet<String> = HashSet::new();

            // Collect existing display names using consolidated function
            for membership in &existing_memberships {
                if membership.turn_order == Some(seat_to_fill) {
                    continue;
                }

                // Only process players (they have turn_order)
                let Some(seat) = membership.turn_order else {
                    continue;
                };

                let display_name = resolve_display_name_for_membership(
                    txn, membership, seat,
                    false, // No final fallback needed for uniqueness check
                )
                .await
                .map_err(AppError::from)?;

                existing_display_names.insert(display_name);
            }

            let ai_service = AiService;
            let (factory, registry_version, resolved_seed) =
                resolve_registry_selection(&request, Some(Strategic::NAME))?;

            let mut seed = resolved_seed;
            if seed.is_none() && factory.name == RandomPlayer::NAME {
                seed = Some(random::<u64>());
            }

            // Use the shared default AI lookup when appropriate
            let ai_profile = if factory.name == Strategic::NAME {
                let game_service = GameService;
                game_service.find_default_ai_profile(txn).await?
            } else {
                // For other AI types, use the standard lookup
                ai_profiles::find_by_registry_variant(
                    txn,
                    factory.name,
                    &registry_version,
                    "default",
                )
                .await
                .map_err(AppError::from)?
                .ok_or_else(|| {
                    AppError::bad_request(
                        ErrorCode::ValidationError,
                        format!(
                            "AI profile for {} v{} not found",
                            factory.name, registry_version
                        ),
                    )
                })?
            };

            // Generate a random seed for name generation to ensure names vary between games
            let name_seed = random::<u64>() as i64;
            let base_name = friendly_ai_name(name_seed, seat_index as usize);
            let unique_name = unique_ai_display_name(&existing_display_names, &base_name);
            existing_display_names.insert(unique_name.clone());

            let mut override_config = serde_json::Map::new();
            if let Some(seed_value) = seed {
                override_config.insert(
                    "seed".to_string(),
                    serde_json::Value::Number(seed_value.into()),
                );
            }
            let override_config = if override_config.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(override_config))
            };

            let overrides = AiInstanceOverrides {
                name: Some(unique_name),
                memory_level: None,
                config: override_config,
            };

            // Check if adding this AI will complete all 4 players and start the game
            // This determines whether we need to touch_game after (game won't start) or not (game will start)
            // Count only players (exclude spectators)
            let player_count = existing_memberships
                .iter()
                .filter(|m| m.role == GameRole::Player)
                .count();
            let will_complete_players = player_count == 3;
            let all_existing_ready = existing_memberships
                .iter()
                .filter(|m| m.role == GameRole::Player)
                .all(|m| m.is_ready);
            let game_will_start = will_complete_players && all_existing_ready;

            ai_service
                .add_ai_to_game(txn, id, ai_profile.id, seat_to_fill, Some(overrides))
                .await
                .map_err(AppError::from)?;

            // Only touch game if it didn't start (not all 4 players ready, or not all existing were ready)
            // If game started, version was already updated by deal_round/process_game_state
            if !game_will_start {
                // Use version from request (consistent with leave_game pattern)
                // The game state hasn't changed (still in Lobby), so version is still valid
                let updated_game = games_repo::touch_game(txn, id, _version)
                    .await
                    .map_err(AppError::from)?;
                Ok(updated_game.version)
            } else {
                // Game started - fetch to get the updated version from deal_round/process_game_state
                let game = games_repo::require_game(txn, id).await?;
                Ok(game.version)
            }
        })
    })
    .await?;

    let etag = game_etag(id, updated_version);

    // Publish snapshot broadcast so other players' UIs update
    publish_snapshot(&app_state, id).await?;

    Ok(HttpResponse::NoContent()
        .insert_header((ETAG, etag))
        .finish())
}

async fn remove_ai_seat(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: ValidatedJson<ManageAiSeatRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let request = body.into_inner();
    let requested_seat = request.seat;
    let host_user_id = membership.user_id;
    let version = request.version;

    let updated_version = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let game = games_repo::require_game(txn, id).await?;

            let game_service = GameService;
            if !game_service.is_host(&game, host_user_id) {
                return Err(AppError::forbidden_with_code(
                    ErrorCode::Forbidden,
                    "Only the host can manage AI seats",
                ));
            }

            if game.state != GameState::Lobby {
                return Err(AppError::bad_request(
                    ErrorCode::PhaseMismatch,
                    "AI seats can only be managed before the game starts",
                ));
            }

            let existing_memberships = memberships::find_all_by_game(txn, id)
                .await
                .map_err(AppError::from)?;

            let candidate = if let Some(seat_val) = requested_seat {
                if seat_val > 3 {
                    return Err(AppError::bad_request(
                        ErrorCode::InvalidSeat,
                        format!("Seat {seat_val} is out of range (0-3)"),
                    ));
                }

                let membership = existing_memberships
                    .iter()
                    .find(|m| m.turn_order == Some(seat_val))
                    .cloned()
                    .ok_or_else(|| {
                        AppError::bad_request(
                            ErrorCode::ValidationError,
                            format!("No player assigned to seat {seat_val}"),
                        )
                    })?;

                if membership.player_kind != crate::entities::game_players::PlayerKind::Ai {
                    return Err(AppError::bad_request(
                        ErrorCode::ValidationError,
                        "Specified seat is not occupied by an AI player",
                    ));
                }

                membership
            } else {
                let mut ai_memberships = Vec::new();

                for member in &existing_memberships {
                    if member.player_kind == crate::entities::game_players::PlayerKind::Ai {
                        ai_memberships.push(member.clone());
                    }
                }

                if ai_memberships.is_empty() {
                    return Err(AppError::bad_request(
                        ErrorCode::ValidationError,
                        "There are no AI seats to remove",
                    ));
                }

                ai_memberships
                    .into_iter()
                    .max_by_key(|m| m.turn_order)
                    .ok_or_else(|| {
                        AppError::internal(
                            ErrorCode::InternalError,
                            "Failed to select AI membership for removal",
                            std::io::Error::other("No AI membership found"),
                        )
                    })?
            };

            if candidate.player_kind != crate::entities::game_players::PlayerKind::Ai {
                return Err(AppError::bad_request(
                    ErrorCode::ValidationError,
                    "Specified seat is not occupied by an AI player",
                ));
            }

            ai_overrides::delete_by_game_player_id(txn, candidate.id)
                .await
                .map_err(AppError::from)?;
            memberships::delete_membership(txn, candidate.id)
                .await
                .map_err(AppError::from)?;

            // Touch game to increment version so websocket clients receive the update
            let updated_game = games_repo::touch_game(txn, id, version)
                .await
                .map_err(AppError::from)?;
            Ok(updated_game.version)
        })
    })
    .await?;

    let etag = game_etag(id, updated_version);

    // Publish snapshot broadcast so other players' UIs update
    publish_snapshot(&app_state, id).await?;

    Ok(HttpResponse::NoContent()
        .insert_header((ETAG, etag))
        .finish())
}

async fn update_ai_seat(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: ValidatedJson<ManageAiSeatRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let request = body.into_inner();

    let Some(seat_val) = request.seat else {
        return Err(AppError::bad_request(
            ErrorCode::ValidationError,
            "seat is required".to_string(),
        ));
    };

    if !(0..=3).contains(&seat_val) {
        return Err(AppError::bad_request(
            ErrorCode::InvalidSeat,
            format!("Seat {seat_val} is out of range (0-3)"),
        ));
    }

    let version = request.version;

    let updated_version = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let game = games_repo::require_game(txn, id).await?;

            let host_user_id = membership.user_id;

            let game_service = GameService;
            if !game_service.is_host(&game, host_user_id) {
                return Err(AppError::forbidden_with_code(
                    ErrorCode::Forbidden,
                    "Only the host can manage AI seats",
                ));
            }

            if game.state != GameState::Lobby {
                return Err(AppError::bad_request(
                    ErrorCode::PhaseMismatch,
                    "AI seats can only be managed before the game starts",
                ));
            }

            let existing_memberships = memberships::find_all_by_game(txn, id)
                .await
                .map_err(AppError::from)?;

            let membership = existing_memberships
                .into_iter()
                .find(|m| m.turn_order == Some(seat_val))
                .ok_or_else(|| {
                    AppError::bad_request(
                        ErrorCode::ValidationError,
                        format!("No player assigned to seat {seat_val}"),
                    )
                })?;

            if membership.player_kind != crate::entities::game_players::PlayerKind::Ai {
                return Err(AppError::bad_request(
                    ErrorCode::ValidationError,
                    "Cannot update AI profile for a human player",
                ));
            }

            let (factory, registry_version, resolved_seed) =
                resolve_registry_selection(&request, None)?;

            let mut seed = resolved_seed;
            if seed.is_none() && factory.name == RandomPlayer::NAME {
                seed = Some(random::<u64>());
            }

            let new_profile = ai_profiles::find_by_registry_variant(
                txn,
                factory.name,
                &registry_version,
                "default",
            )
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| {
                AppError::bad_request(
                    ErrorCode::ValidationError,
                    format!(
                        "AI profile for {} v{} not found",
                        factory.name, registry_version
                    ),
                )
            })?;

            let mut updated_membership = membership.clone();
            updated_membership.ai_profile_id = Some(new_profile.id);
            memberships::update_membership(txn, updated_membership)
                .await
                .map_err(AppError::from)?;

            if let Some(seed_value) = seed {
                let mut cfg = serde_json::Map::new();
                cfg.insert(
                    "seed".to_string(),
                    serde_json::Value::Number(seed_value.into()),
                );
                let cfg_value = serde_json::Value::Object(cfg);
                if let Some(mut existing_override) =
                    ai_overrides::find_by_game_player_id(txn, membership.id)
                        .await
                        .map_err(AppError::from)?
                {
                    existing_override.config = Some(cfg_value);
                    ai_overrides::update_override(txn, existing_override)
                        .await
                        .map_err(AppError::from)?;
                } else {
                    ai_overrides::create_override(txn, membership.id, None, None, Some(cfg_value))
                        .await
                        .map_err(AppError::from)?;
                }
            }

            // Touch game to increment version so websocket clients receive the update
            let updated_game = games_repo::touch_game(txn, id, version)
                .await
                .map_err(AppError::from)?;
            Ok(updated_game.version)
        })
    })
    .await?;

    let etag = game_etag(id, updated_version);

    // Publish snapshot broadcast so other players' UIs update
    publish_snapshot(&app_state, id).await?;

    Ok(HttpResponse::NoContent()
        .insert_header((ETAG, etag))
        .finish())
}

/// POST /api/games/{game_id}/rejoin
///
/// Rejoin a game by converting an AI player back to the original human player.
/// Requires version in request body for optimistic locking.
async fn rejoin_game(
    http_req: HttpRequest,
    game_id: GameId,
    current_user: CurrentUser,
    body: ValidatedJson<RejoinGameRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let user_id = current_user.id;
    let id = game_id.0;
    let expected_version = body.version;

    // Rejoin game using service layer
    let updated_version = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameService;
            let (updated_game, _) = service
                .rejoin_game(txn, id, user_id, expected_version)
                .await?;
            Ok(updated_game.version)
        })
    })
    .await?;

    let etag = game_etag(id, updated_version);

    // Publish snapshot broadcast immediately (includes membership change)
    publish_snapshot(&app_state, id).await?;

    // Spawn background task to process any pending AI actions
    // If AI was acting, it will complete, then see player is human and stop
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        let db = match crate::db::require_db(&app_state_clone) {
            Ok(db) => db,
            Err(err) => {
                tracing::error!(
                    game_id = id,
                    error = %err,
                    "Failed to get database connection for post-rejoin AI processing"
                );
                return;
            }
        };

        let txn = match db.begin().await {
            Ok(txn) => txn,
            Err(err) => {
                tracing::error!(
                    game_id = id,
                    error = %err,
                    "Failed to begin transaction for post-rejoin AI processing"
                );
                return;
            }
        };

        let game_flow_service = crate::services::game_flow::GameFlowService;
        match game_flow_service.process_game_state(&txn, id).await {
            Ok(()) => {
                if let Err(err) = txn.commit().await {
                    tracing::error!(
                        game_id = id,
                        error = %err,
                        "Failed to commit transaction after post-rejoin AI processing"
                    );
                } else {
                    // Broadcast updated snapshot after AI processing completes
                    if let Err(err) = publish_snapshot(&app_state_clone, id).await {
                        tracing::error!(
                            game_id = id,
                            error = %err,
                            "Failed to publish snapshot after post-rejoin AI processing"
                        );
                    }
                }
            }
            Err(err) => {
                tracing::warn!(
                    game_id = id,
                    error = %err,
                    "Post-rejoin AI processing failed (may be expected if no AI actions)"
                );
                // Rollback on error
                if let Err(rollback_err) = txn.rollback().await {
                    tracing::error!(
                        game_id = id,
                        error = %rollback_err,
                        "Failed to rollback transaction after post-rejoin AI processing error"
                    );
                }
            }
        }
    });

    Ok(HttpResponse::Ok()
        .insert_header((ETAG, etag))
        .json(json!({ "version": updated_version })))
}

/// POST /api/games/{game_id}/bid
///
/// Submits a bid for the current player. Bidding order and validation are enforced
/// by the service layer.
///
/// Requires version in request body for optimistic locking.
async fn submit_bid(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: ValidatedJson<SubmitBidRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    // Only players can submit bids
    if membership.role != GameRole::Player {
        return Err(AppError::forbidden_with_code(
            ErrorCode::InsufficientRole,
            "Only players can submit bids".to_string(),
        ));
    }

    let id = game_id.0;
    let seat = membership.turn_order.ok_or_else(|| {
        AppError::bad_request(
            crate::errors::ErrorCode::InvalidSeat,
            "Player must have a seat to perform this action".to_string(),
        )
    })?;
    let bid_value = body.bid;

    let game = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameFlowService;
            service
                .submit_bid(txn, id, seat, bid_value, body.version)
                .await
        })
    })
    .await?;

    let etag = game_etag(game.id, game.version);
    publish_snapshot_with_lock(&app_state, id, game.version).await?;
    Ok(HttpResponse::NoContent()
        .insert_header((ETAG, etag))
        .finish())
}

/// Sets the trump suit for the current round. Only the winning bidder can set trump.
///
/// Requires version in request body for optimistic locking.
async fn select_trump(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: ValidatedJson<SetTrumpRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    // Only players can select trump
    if membership.role != GameRole::Player {
        return Err(AppError::forbidden_with_code(
            ErrorCode::InsufficientRole,
            "Only players can select trump".to_string(),
        ));
    }

    let id = game_id.0;
    let seat = membership.turn_order.ok_or_else(|| {
        AppError::bad_request(
            crate::errors::ErrorCode::InvalidSeat,
            "Player must have a seat to perform this action".to_string(),
        )
    })?;
    let payload = body.into_inner();
    let normalized = payload.trump.trim().to_uppercase();

    let trump = match normalized.as_str() {
        "CLUBS" => crate::domain::Trump::Clubs,
        "DIAMONDS" => crate::domain::Trump::Diamonds,
        "HEARTS" => crate::domain::Trump::Hearts,
        "SPADES" => crate::domain::Trump::Spades,
        "NO_TRUMPS" => crate::domain::Trump::NoTrumps,
        _ => {
            return Err(AppError::bad_request(
                ErrorCode::ValidationError,
                format!("Invalid trump value: {}", payload.trump),
            ))
        }
    };

    let game = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameFlowService;
            service
                .set_trump(txn, id, seat, trump, payload.version)
                .await
        })
    })
    .await?;

    let etag = game_etag(game.id, game.version);
    publish_snapshot_with_lock(&app_state, id, game.version).await?;
    Ok(HttpResponse::NoContent()
        .insert_header((ETAG, etag))
        .finish())
}

/// Plays a card for the current player in the current trick.
///
/// Requires version in request body for optimistic locking.
async fn play_card(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: ValidatedJson<PlayCardRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    // Only players can play cards
    if membership.role != GameRole::Player {
        return Err(AppError::forbidden_with_code(
            ErrorCode::InsufficientRole,
            "Only players can play cards".to_string(),
        ));
    }

    let id = game_id.0;
    let seat = membership.turn_order.ok_or_else(|| {
        AppError::bad_request(
            crate::errors::ErrorCode::InvalidSeat,
            "Player must have a seat to perform this action".to_string(),
        )
    })?;

    let payload = body.into_inner();
    let normalized = payload.card.trim().to_uppercase();

    if normalized.is_empty() {
        return Err(AppError::bad_request(
            ErrorCode::ValidationError,
            "Card value is required",
        ));
    }

    let card = normalized.parse::<Card>().map_err(AppError::from)?;

    let game = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameFlowService;
            service
                .play_card(txn, id, seat, card, payload.version)
                .await
        })
    })
    .await?;

    let etag = game_etag(game.id, game.version);
    publish_snapshot_with_lock(&app_state, id, game.version).await?;
    Ok(HttpResponse::NoContent()
        .insert_header((ETAG, etag))
        .finish())
}

/// DELETE /api/games/{game_id}
///
/// Deletes a game. Only the host can delete a game.
///
/// Requires version in request body for optimistic locking.
async fn delete_game(
    http_req: HttpRequest,
    game_id: GameId,
    current_user: CurrentUser,
    body: ValidatedJson<DeleteGameRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let user_id = current_user.id;
    let version = body.version;

    with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            // Load game to verify it exists, check host authorization, and validate lock version
            let game = games_repo::require_game(txn, id).await?;

            // Check host authorization (based on created_by, not membership)
            let game_service = GameService;
            if !game_service.is_host(&game, Some(user_id)) {
                return Err(AppError::forbidden_with_code(
                    ErrorCode::Forbidden,
                    "Only the host can delete a game",
                ));
            }

            // Delete the game with optimistic locking
            // Cascade delete will handle related records automatically
            games_repo::delete_game(txn, id, version)
                .await
                .map_err(AppError::from)?;

            Ok(())
        })
    })
    .await?;

    // DELETE responses should not include ETag headers per HTTP/REST best practices
    // since the resource no longer exists after deletion
    Ok(HttpResponse::NoContent().finish())
}

fn unique_ai_display_name(existing: &HashSet<String>, base: &str) -> String {
    if !existing.contains(base) {
        return base.to_string();
    }

    let mut counter = 2;
    loop {
        let candidate = format!("{base} {counter}");
        if !existing.contains(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}

fn format_card(card: Card) -> String {
    let rank_char = match card.rank {
        Rank::Two => '2',
        Rank::Three => '3',
        Rank::Four => '4',
        Rank::Five => '5',
        Rank::Six => '6',
        Rank::Seven => '7',
        Rank::Eight => '8',
        Rank::Nine => '9',
        Rank::Ten => 'T',
        Rank::Jack => 'J',
        Rank::Queen => 'Q',
        Rank::King => 'K',
        Rank::Ace => 'A',
    };

    let suit_char = match card.suit {
        Suit::Clubs => 'C',
        Suit::Diamonds => 'D',
        Suit::Hearts => 'H',
        Suit::Spades => 'S',
    };

    format!("{rank_char}{suit_char}")
}

fn trump_to_api_value(trump: Trump) -> &'static str {
    match trump {
        Trump::Clubs => "CLUBS",
        Trump::Diamonds => "DIAMONDS",
        Trump::Hearts => "HEARTS",
        Trump::Spades => "SPADES",
        Trump::NoTrumps => "NO_TRUMPS",
    }
}

/// Publish a snapshot broadcast with a known version.
///
/// This is the core function that actually publishes to Redis.
/// Use this when you already have the version (e.g., from a transaction return value).
/// Publish a snapshot broadcast (best-effort).
///
/// This function logs errors but does not fail the HTTP request.
/// The database mutation has already succeeded, so the user's action
/// should succeed even if Redis is temporarily unavailable.
///
/// Clients will receive updates via:
/// - WebSocket when Redis recovers (subscriber reconnects automatically)
/// - Next HTTP poll/refresh
/// - WebSocket reconnect
async fn publish_snapshot_with_lock(
    app_state: &web::Data<AppState>,
    game_id: i64,
    version: i32,
) -> Result<(), AppError> {
    // Invalidate old version from cache (version just incremented, so old version is version - 1)
    // This is a safety measure - we also invalidate right after touch_game where possible
    if version > 1 {
        app_state.snapshot_cache().remove((game_id, version - 1));
    }

    if let Some(realtime) = &app_state.realtime {
        if let Err(err) = realtime.publish_snapshot(game_id, version).await {
            tracing::error!(
                game_id,
                version,
                error = %err,
                "Failed to publish snapshot broadcast (Redis may be unavailable). Mutation succeeded, but clients may not receive real-time updates until Redis recovers."
            );
        }
    }
    Ok(())
}

/// Publish a snapshot broadcast by fetching the game's current version.
///
/// This is a convenience wrapper that fetches the game from the database
/// to get the current version, then calls `publish_snapshot_with_lock`.
///
/// In tests with SharedTxn, the game may not be visible to pooled connections,
/// so prefer `publish_snapshot_with_lock` when you already have the version.
async fn publish_snapshot(app_state: &web::Data<AppState>, game_id: i64) -> Result<(), AppError> {
    // Fetch game to get current version for broadcast
    let db = match crate::db::require_db(app_state) {
        Ok(db) => db,
        Err(_) => {
            // Database not available (e.g., in some test scenarios)
            return Ok(());
        }
    };

    // Try to fetch the game, but don't fail if it doesn't exist (e.g., in tests with SharedTxn)
    let game = match games_repo::find_by_id(db, game_id).await {
        Ok(Some(game)) => game,
        Ok(None) => {
            // Game not found - likely in a test with uncommitted SharedTxn
            return Ok(());
        }
        Err(e) => {
            // Other error - propagate it
            return Err(AppError::from(e));
        }
    };

    publish_snapshot_with_lock(app_state, game_id, game.version).await
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("").route(web::post().to(create_game)));
    cfg.service(web::resource("/joinable").route(web::get().to(list_joinable_games)));
    cfg.service(web::resource("/overview").route(web::get().to(list_overview_games)));
    cfg.service(web::resource("/in-progress").route(web::get().to(list_in_progress_games)));
    cfg.service(web::resource("/last-active").route(web::get().to(get_last_active_game)));
    cfg.service(web::resource("/{game_id}/join").route(web::post().to(join_game)));
    cfg.service(web::resource("/{game_id}/spectate").route(web::post().to(spectate_game)));
    cfg.service(web::resource("/{game_id}/leave").route(web::delete().to(leave_game)));
    cfg.service(web::resource("/{game_id}/rejoin").route(web::post().to(rejoin_game)));
    cfg.service(web::resource("/{game_id}/ready").route(web::post().to(mark_ready)));
    cfg.service(web::resource("/ai/registry").route(web::get().to(list_registered_ais)));
    cfg.service(web::resource("/{game_id}/ai/add").route(web::post().to(add_ai_seat)));
    cfg.service(web::resource("/{game_id}/ai/update").route(web::post().to(update_ai_seat)));
    cfg.service(web::resource("/{game_id}/ai/remove").route(web::post().to(remove_ai_seat)));
    cfg.service(web::resource("/{game_id}/bid").route(web::post().to(submit_bid)));
    cfg.service(web::resource("/{game_id}/trump").route(web::post().to(select_trump)));
    cfg.service(web::resource("/{game_id}/play").route(web::post().to(play_card)));
    cfg.service(web::resource("/{game_id}/snapshot").route(web::get().to(get_snapshot)));
    cfg.service(web::resource("/{game_id}/history").route(web::get().to(get_game_history)));
    cfg.service(
        web::resource("/{game_id}/players/{seat}/display_name")
            .route(web::get().to(get_player_display_name)),
    );
    cfg.service(web::resource("/{game_id}").route(web::delete().to(delete_game)));
}

#[cfg(test)]
mod display_name_tests {
    use std::collections::HashSet;

    use super::unique_ai_display_name;

    #[test]
    fn unique_ai_names_append_suffixes() {
        let mut existing = HashSet::new();
        existing.insert("Atlas Bot".to_string());
        let second = unique_ai_display_name(&existing, "Atlas Bot");
        assert_eq!(second, "Atlas Bot 2");
        existing.insert(second.clone());
        let third = unique_ai_display_name(&existing, "Atlas Bot");
        assert_eq!(third, "Atlas Bot 3");
    }
}
