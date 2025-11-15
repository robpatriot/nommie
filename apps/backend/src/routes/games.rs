//! Game-related HTTP routes.

use std::collections::HashSet;

use actix_web::http::header::{ETAG, IF_NONE_MATCH};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::adapters::games_sea::{self, GameUpdateRound};
use crate::ai::{registry, AiConfig, HeuristicV1, RandomPlayer};
use crate::db::txn::with_txn;
use crate::domain::bidding::validate_consecutive_zero_bids;
use crate::domain::player_view::{CurrentRoundInfo, GameHistory};
use crate::domain::snapshot::{GameSnapshot, SeatAiProfilePublic, SeatPublic};
use crate::domain::state::Seat;
use crate::domain::{Card, Rank, Suit};
use crate::entities::games::{self, GameState, GameVisibility};
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::extractors::current_user::CurrentUser;
use crate::extractors::game_id::GameId;
use crate::extractors::game_membership::GameMembership;
use crate::extractors::ValidatedJson;
use crate::http::etag::{game_etag, ExpectedVersion};
use crate::repos::memberships::{self, GameRole};
use crate::repos::{ai_overrides, ai_profiles, rounds, users};
use crate::services::ai::AiService;
use crate::services::game_flow::GameFlowService;
use crate::services::games::GameService;
use crate::services::players::PlayerService;
use crate::state::app_state::AppState;

#[derive(Serialize)]
struct GameSnapshotResponse {
    snapshot: GameSnapshot,
    viewer_hand: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bid_constraints: Option<BidConstraintsResponse>,
}

#[derive(Serialize)]
struct BidConstraintsResponse {
    zero_bid_locked: [bool; 4],
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
    let (snapshot, lock_version, viewer_seat, viewer_hand, bid_constraints) =
        with_txn(Some(&http_req), &app_state, |txn| {
            Box::pin(async move {
                // Fetch game from database to get lock_version
                let game = games_sea::find_by_id(txn, id)
                    .await
                    .map_err(|e| AppError::db("failed to fetch game", e))?
                    .ok_or_else(|| {
                        AppError::not_found(
                            crate::errors::ErrorCode::GameNotFound,
                            format!("Game with ID {id} not found"),
                        )
                    })?;

                // Create game service and load game state
                let game_service = crate::services::games::GameService;
                let state = game_service.load_game_state(txn, id).await?;

                let memberships = memberships::find_all_by_game(txn, id)
                    .await
                    .map_err(AppError::from)?;

                let mut viewer_seat: Option<Seat> = None;
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
                    let seat_idx = membership.turn_order;
                    if !(0..4).contains(&seat_idx) {
                        continue;
                    }

                    let seat = seat_idx as u8;
                    let mut seat_public = SeatPublic::empty(seat);
                    seat_public.is_ready = membership.is_ready;
                    seat_public.user_id = Some(membership.user_id);

                    let user = users::find_user_by_id(txn, membership.user_id)
                        .await
                        .map_err(AppError::from)?;
                    let ai_override = ai_overrides::find_by_game_player_id(txn, membership.id)
                        .await
                        .map_err(AppError::from)?;
                    let ai_profile = ai_profiles::find_by_user_id(txn, membership.user_id)
                        .await
                        .map_err(AppError::from)?;

                    if let Some(user) = user.as_ref() {
                        seat_public.is_ai = user.is_ai;
                    }

                    let mut display_name: Option<String> = ai_override
                        .as_ref()
                        .and_then(|ovr| ovr.name.as_ref())
                        .and_then(|name| {
                            let trimmed = name.trim();
                            if trimmed.is_empty() {
                                None
                            } else {
                                Some(trimmed.to_owned())
                            }
                        });

                    if display_name.is_none() {
                        if let Some(user_ref) = user.as_ref() {
                            if user_ref.is_ai {
                                if let Some(profile) = ai_profile.as_ref() {
                                    let trimmed = profile.display_name.trim();
                                    if !trimmed.is_empty() {
                                        display_name = Some(trimmed.to_owned());
                                    }
                                }
                            } else if let Some(username) = &user_ref.username {
                                let trimmed = username.trim();
                                if !trimmed.is_empty() {
                                    display_name = Some(trimmed.to_owned());
                                }
                            }
                        }
                    }

                    if seat_public.is_ai {
                        if let Some(profile) = ai_profile.as_ref() {
                            if let Some(ai_name) = profile.playstyle.clone() {
                                let config = AiConfig::from_json(profile.config.as_ref());
                                let version = config
                                    .get_custom_str("registry_version")
                                    .map(|v| v.to_owned())
                                    .or_else(|| {
                                        registry::by_name(&ai_name)
                                            .map(|factory| factory.version.to_owned())
                                    })
                                    .unwrap_or_else(|| "unknown".to_string());

                                seat_public.ai_profile = Some(SeatAiProfilePublic {
                                    name: ai_name,
                                    version,
                                });
                            }
                        }
                    }

                    if display_name.is_none() {
                        if let Some(user_ref) = user.as_ref() {
                            if user_ref.is_ai {
                                display_name = Some(friendly_ai_name(user_ref.id, seat as usize));
                            }
                        }
                    }

                    if display_name.is_none() {
                        display_name = Some(format!("Player {}", seat as usize + 1));
                    }

                    seat_public.display_name = display_name;

                    seating[seat_idx as usize] = seat_public;

                    if creator_user_id == Some(membership.user_id) {
                        host_seat = seat;
                    }

                    if membership.user_id == current_user.id {
                        viewer_seat = Some(seat);
                    }
                }

                snap.game.seating = seating;
                snap.game.host_seat = host_seat;

                let viewer_hand = if let Some(seat) = viewer_seat {
                    if state.round_no == 0 {
                        None
                    } else {
                        match CurrentRoundInfo::load(txn, id, seat as i16).await {
                            Ok(info) => {
                                Some(info.hand.into_iter().map(format_card).collect::<Vec<_>>())
                            }
                            Err(err) => {
                                warn!(
                                    game_id = id,
                                    seat,
                                    error = %err,
                                    "Failed to load viewer hand"
                                );
                                None
                            }
                        }
                    }
                } else {
                    None
                };

                let mut bid_constraints: Option<BidConstraintsResponse> = None;
                if matches!(
                    &snap.phase,
                    crate::domain::snapshot::PhaseSnapshot::Bidding(_)
                ) {
                    if let Some(current_round_no) = game.current_round {
                        let history = GameHistory::load(txn, id).await?;
                        let mut zero_bid_locked = [false; 4];
                        for seat in 0..4 {
                            if validate_consecutive_zero_bids(
                                &history,
                                seat as i16,
                                current_round_no,
                            )
                            .is_err()
                            {
                                zero_bid_locked[seat as usize] = true;
                            }
                        }
                        bid_constraints = Some(BidConstraintsResponse { zero_bid_locked });
                    }
                }

                Ok((
                    snap,
                    game.lock_version,
                    viewer_seat,
                    viewer_hand,
                    bid_constraints,
                ))
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
                let mut not_modified = HttpResponse::build(StatusCode::NOT_MODIFIED);
                not_modified.insert_header((ETAG, etag_value));
                if let Some(seat) = viewer_seat {
                    not_modified.insert_header(("x-viewer-seat", seat.to_string()));
                }
                return Ok(not_modified.finish());
            }
        }
    }

    // Resource is new or modified, return full response
    let mut response = HttpResponse::Ok();
    response.insert_header((ETAG, etag_value));
    if let Some(seat) = viewer_seat {
        response.insert_header(("x-viewer-seat", seat.to_string()));
    }
    Ok(response.json(GameSnapshotResponse {
        snapshot,
        viewer_hand,
        bid_constraints,
    }))
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
            let service = PlayerService;
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
    current_round: Option<i16>,
    player_count: i32,
    max_players: i32,
    viewer_is_member: bool,
    viewer_is_host: bool,
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

/// Helper function to convert game entity + memberships to frontend Game format
fn game_to_response(
    game_model: &crate::entities::games::Model,
    memberships: &[memberships::GameMembership],
    viewer_user_id: Option<i64>,
) -> GameResponse {
    // Count players (exclude spectators)
    let player_count = memberships
        .iter()
        .filter(|m| m.role == GameRole::Player)
        .count() as i32;

    let viewer_is_member = viewer_user_id
        .and_then(|id| {
            memberships
                .iter()
                .find(|m| m.user_id == id && m.role == GameRole::Player)
        })
        .is_some();

    // Check if viewer is the host (creator of the game)
    let viewer_is_host = viewer_user_id
        .and_then(|id| game_model.created_by.map(|created_by| created_by == id))
        .unwrap_or(false);

    GameResponse {
        id: game_model.id,
        name: game_model
            .name
            .clone()
            .unwrap_or_else(|| format!("Game {}", game_model.id)),
        state: game_state_to_string(&game_model.state),
        visibility: game_visibility_to_string(&game_model.visibility),
        created_by: game_model.created_by.unwrap_or(0),
        created_at: game_model.created_at.to_string(),
        updated_at: game_model.updated_at.to_string(),
        started_at: game_model.started_at.map(|dt| dt.to_string()),
        ended_at: game_model.ended_at.map(|dt| dt.to_string()),
        current_round: game_model.current_round,
        player_count,
        max_players: 4,
        viewer_is_member,
        viewer_is_host,
    }
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

    let response = game_to_response(&game_model, &memberships, Some(user_id));

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
            let service = GameService;
            service.join_game(txn, id, user_id).await
        })
    })
    .await?;

    let response = game_to_response(&game_model, &memberships, Some(user_id));

    Ok(web::Json(JoinGameResponse { game: response }))
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
        .collect();

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
        let viewer_is_member = memberships.iter().any(|m| m.user_id == viewer_id);

        if game_model.visibility == GameVisibility::Public || viewer_is_member {
            visible_games.push(game_to_response(&game_model, &memberships, Some(viewer_id)));
        }
    }

    Ok(web::Json(GameListResponse {
        games: visible_games,
    }))
}

/// GET /api/games/last-active
///
/// Returns the game ID of the most recently active game for the current user.
/// "Most recently active" means the game with the highest updated_at timestamp
/// among all games where the user is a member.
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

/// POST /api/games/{game_id}/ready
///
/// Marks the current player as ready. When all four seats are ready, the game
/// automatically transitions into the first round.
async fn mark_ready(
    http_req: HttpRequest,
    game_id: GameId,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let user_id = current_user.id;

    with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameFlowService;
            service.mark_ready(txn, id, user_id).await?;
            Ok(())
        })
    })
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(serde::Deserialize)]
struct SubmitBidRequest {
    bid: u8,
}

#[derive(serde::Deserialize)]
struct SetTrumpRequest {
    trump: String,
}

#[derive(serde::Deserialize)]
struct PlayCardRequest {
    card: String,
}

#[derive(Debug, Default, Deserialize)]
struct ManageAiSeatRequest {
    seat: Option<u8>,
    registry_name: Option<String>,
    registry_version: Option<String>,
    seed: Option<u64>,
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

fn build_ai_profile_config(
    registry_name: &str,
    registry_version: &str,
    seed: Option<u64>,
) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert(
        "registry_name".to_string(),
        serde_json::Value::String(registry_name.to_string()),
    );
    obj.insert(
        "registry_version".to_string(),
        serde_json::Value::String(registry_version.to_string()),
    );
    if let Some(seed_value) = seed {
        obj.insert(
            "seed".to_string(),
            serde_json::Value::Number(seed_value.into()),
        );
    }
    serde_json::Value::Object(obj)
}

async fn add_ai_seat(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: Option<web::Json<ManageAiSeatRequest>>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let request = body.map(|b| b.into_inner()).unwrap_or_default();
    let requested_seat = request.seat;
    let host_user_id = membership.user_id;
    let host_turn_order = membership.turn_order;

    with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let game = games_sea::find_by_id(txn, id)
                .await
                .map_err(|e| AppError::db("failed to fetch game", e))?
                .ok_or_else(|| {
                    AppError::not_found(
                        ErrorCode::GameNotFound,
                        format!("Game with ID {id} not found"),
                    )
                })?;

            let is_host = match game.created_by {
                Some(created_by) => created_by == host_user_id,
                None => host_turn_order == 0,
            };

            if !is_host {
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

            if existing_memberships.len() >= 4 {
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
                    .any(|m| m.turn_order == seat_val as i32)
                {
                    return Err(AppError::conflict(
                        ErrorCode::SeatTaken,
                        format!("Seat {seat_val} is already taken"),
                    ));
                }
                seat_val as i32
            } else {
                let game_service = GameService;
                game_service
                    .find_next_available_seat(&existing_memberships)
                    .ok_or_else(|| {
                        AppError::conflict(ErrorCode::SeatTaken, "No available seats remaining")
                    })?
            };

            let seat_index = seat_to_fill as u8;
            let mut existing_display_names: HashSet<String> = HashSet::new();

            for membership in &existing_memberships {
                if membership.turn_order == seat_to_fill {
                    continue;
                }

                let user = users::find_user_by_id(txn, membership.user_id)
                    .await
                    .map_err(AppError::from)?;

                if let Some(user) = user {
                    if let Some(override_record) =
                        ai_overrides::find_by_game_player_id(txn, membership.id)
                            .await
                            .map_err(AppError::from)?
                    {
                        if let Some(name) = override_record.name {
                            let trimmed = name.trim();
                            if !trimmed.is_empty() {
                                existing_display_names.insert(trimmed.to_owned());
                                continue;
                            }
                        }
                    }

                    if user.is_ai {
                        if let Some(profile) = ai_profiles::find_by_user_id(txn, user.id)
                            .await
                            .map_err(AppError::from)?
                        {
                            let trimmed = profile.display_name.trim();
                            if !trimmed.is_empty() {
                                existing_display_names.insert(trimmed.to_owned());
                            } else {
                                existing_display_names.insert(friendly_ai_name(
                                    user.id,
                                    membership.turn_order as usize,
                                ));
                            }
                        } else {
                            existing_display_names
                                .insert(friendly_ai_name(user.id, membership.turn_order as usize));
                        }
                    } else if let Some(username) = &user.username {
                        let trimmed = username.trim();
                        if !trimmed.is_empty() {
                            existing_display_names.insert(trimmed.to_owned());
                        }
                    }
                }
            }

            let ai_service = AiService;
            let (factory, registry_version, resolved_seed) =
                resolve_registry_selection(&request, Some(HeuristicV1::NAME))?;

            let mut seed = resolved_seed;
            if seed.is_none() && factory.name == RandomPlayer::NAME {
                seed = Some(random::<u64>());
            }

            let ai_config = build_ai_profile_config(factory.name, &registry_version, seed);

            let ai_user_id = ai_service
                .create_ai_template_user(
                    txn,
                    format!("Bot {}", seat_index + 1),
                    factory.name,
                    &registry_version,
                    Some(ai_config),
                    Some(100),
                )
                .await
                .map_err(AppError::from)?;

            if let Some(mut profile) = ai_profiles::find_by_user_id(txn, ai_user_id)
                .await
                .map_err(AppError::from)?
            {
                let base_name = friendly_ai_name(ai_user_id, seat_index as usize);
                let unique_name = unique_ai_display_name(&existing_display_names, &base_name);
                existing_display_names.insert(unique_name.clone());
                profile.display_name = unique_name;
                ai_profiles::update_profile(txn, profile)
                    .await
                    .map_err(AppError::from)?;
            }

            ai_service
                .add_ai_to_game(txn, id, ai_user_id, seat_to_fill, None)
                .await
                .map_err(AppError::from)?;

            games_sea::update_round(txn, GameUpdateRound::new(id, game.lock_version))
                .await
                .map_err(AppError::from)?;

            // Check if all players are ready and start the game if so
            // This handles the case where the host marks themselves ready before adding AI players
            let game_flow_service = GameFlowService;
            game_flow_service
                .check_and_start_game_if_ready(txn, id)
                .await?;

            Ok(())
        })
    })
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

async fn remove_ai_seat(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: Option<web::Json<ManageAiSeatRequest>>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let request = body.map(|b| b.into_inner()).unwrap_or_default();
    let requested_seat = request.seat;
    let host_user_id = membership.user_id;
    let host_turn_order = membership.turn_order;

    with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let game = games_sea::find_by_id(txn, id)
                .await
                .map_err(|e| AppError::db("failed to fetch game", e))?
                .ok_or_else(|| {
                    AppError::not_found(
                        ErrorCode::GameNotFound,
                        format!("Game with ID {id} not found"),
                    )
                })?;

            let is_host = match game.created_by {
                Some(created_by) => created_by == host_user_id,
                None => host_turn_order == 0,
            };

            if !is_host {
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

                let seat_i32 = seat_val as i32;
                let membership = existing_memberships
                    .iter()
                    .find(|m| m.turn_order == seat_i32)
                    .cloned()
                    .ok_or_else(|| {
                        AppError::bad_request(
                            ErrorCode::ValidationError,
                            format!("No player assigned to seat {seat_val}"),
                        )
                    })?;

                let user = users::find_user_by_id(txn, membership.user_id)
                    .await
                    .map_err(AppError::from)?
                    .ok_or_else(|| {
                        AppError::not_found(
                            ErrorCode::UserNotFound,
                            format!("User {} not found", membership.user_id),
                        )
                    })?;

                if !user.is_ai {
                    return Err(AppError::bad_request(
                        ErrorCode::ValidationError,
                        "Specified seat is not occupied by an AI player",
                    ));
                }

                membership
            } else {
                let mut ai_memberships = Vec::new();

                for member in &existing_memberships {
                    if member.user_id == host_user_id {
                        continue;
                    }

                    if let Some(user) = users::find_user_by_id(txn, member.user_id)
                        .await
                        .map_err(AppError::from)?
                    {
                        if user.is_ai {
                            ai_memberships.push(member.clone());
                        }
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

            let candidate_user = users::find_user_by_id(txn, candidate.user_id)
                .await
                .map_err(AppError::from)?
                .ok_or_else(|| {
                    AppError::not_found(
                        ErrorCode::UserNotFound,
                        format!("User {} not found", candidate.user_id),
                    )
                })?;

            if !candidate_user.is_ai {
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

            games_sea::update_round(txn, GameUpdateRound::new(id, game.lock_version))
                .await
                .map_err(AppError::from)?;

            Ok(())
        })
    })
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

async fn update_ai_seat(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: Option<web::Json<ManageAiSeatRequest>>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let request = body.map(|b| b.into_inner()).unwrap_or_default();

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

    with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let game = games_sea::find_by_id(txn, id)
                .await
                .map_err(|e| AppError::db("failed to fetch game", e))?
                .ok_or_else(|| {
                    AppError::not_found(
                        ErrorCode::GameNotFound,
                        format!("Game with ID {id} not found"),
                    )
                })?;

            let host_user_id = membership.user_id;
            let host_turn_order = membership.turn_order;

            let is_host = match game.created_by {
                Some(created_by) => created_by == host_user_id,
                None => host_turn_order == 0,
            };

            if !is_host {
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

            let seat_i32 = seat_val as i32;
            let membership = existing_memberships
                .into_iter()
                .find(|m| m.turn_order == seat_i32)
                .ok_or_else(|| {
                    AppError::bad_request(
                        ErrorCode::ValidationError,
                        format!("No player assigned to seat {seat_val}"),
                    )
                })?;

            let user = users::find_user_by_id(txn, membership.user_id)
                .await
                .map_err(AppError::from)?
                .ok_or_else(|| {
                    AppError::not_found(
                        ErrorCode::UserNotFound,
                        format!("User {} not found", membership.user_id),
                    )
                })?;

            if !user.is_ai {
                return Err(AppError::bad_request(
                    ErrorCode::ValidationError,
                    "Cannot update AI profile for a human player",
                ));
            }

            let mut profile = ai_profiles::find_by_user_id(txn, user.id)
                .await
                .map_err(AppError::from)?
                .ok_or_else(|| {
                    AppError::bad_request(
                        ErrorCode::ValidationError,
                        format!("AI profile not found for user {}", user.id),
                    )
                })?;

            let (factory, registry_version, resolved_seed) =
                resolve_registry_selection(&request, None)?;

            let mut seed = resolved_seed;
            if seed.is_none() && factory.name == RandomPlayer::NAME {
                seed = Some(random::<u64>());
            }

            let config = build_ai_profile_config(factory.name, &registry_version, seed);

            profile.playstyle = Some(factory.name.to_string());
            profile.config = Some(config);

            ai_profiles::update_profile(txn, profile)
                .await
                .map_err(AppError::from)?;

            games_sea::update_round(txn, GameUpdateRound::new(id, game.lock_version))
                .await
                .map_err(AppError::from)?;

            Ok(())
        })
    })
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

/// POST /api/games/{game_id}/bid
///
/// Submits a bid for the current player. Bidding order and validation are enforced
/// by the service layer.
///
/// Requires If-Match header with the current game ETag for optimistic locking.
async fn submit_bid(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: ValidatedJson<SubmitBidRequest>,
    expected_version: ExpectedVersion,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let seat = membership.turn_order as i16;
    let bid_value = body.bid;

    let updated_game = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameFlowService;
            service
                .submit_bid(txn, id, seat, bid_value, Some(expected_version.0))
                .await
        })
    })
    .await?;

    let etag = game_etag(updated_game.id, updated_game.lock_version);
    Ok(HttpResponse::NoContent()
        .insert_header((ETAG, etag))
        .finish())
}

/// Sets the trump suit for the current round. Only the winning bidder can set trump.
///
/// Requires If-Match header with the current game ETag for optimistic locking.
async fn select_trump(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: ValidatedJson<SetTrumpRequest>,
    expected_version: ExpectedVersion,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let seat = membership.turn_order as i16;
    let payload = body.into_inner();
    let normalized = payload.trump.trim().to_uppercase();

    let trump = match normalized.as_str() {
        "CLUBS" => rounds::Trump::Clubs,
        "DIAMONDS" => rounds::Trump::Diamonds,
        "HEARTS" => rounds::Trump::Hearts,
        "SPADES" => rounds::Trump::Spades,
        "NO_TRUMP" => rounds::Trump::NoTrump,
        _ => {
            return Err(AppError::bad_request(
                ErrorCode::ValidationError,
                format!("Invalid trump value: {}", payload.trump),
            ))
        }
    };

    let updated_game = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameFlowService;
            service
                .set_trump(txn, id, seat, trump, Some(expected_version.0))
                .await
        })
    })
    .await?;

    let etag = game_etag(updated_game.id, updated_game.lock_version);
    Ok(HttpResponse::NoContent()
        .insert_header((ETAG, etag))
        .finish())
}

/// Plays a card for the current player in the current trick.
///
/// Requires If-Match header with the current game ETag for optimistic locking.
async fn play_card(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: ValidatedJson<PlayCardRequest>,
    expected_version: ExpectedVersion,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let seat = membership.turn_order as i16;

    let payload = body.into_inner();
    let normalized = payload.card.trim().to_uppercase();

    if normalized.is_empty() {
        return Err(AppError::bad_request(
            ErrorCode::ValidationError,
            "Card value is required",
        ));
    }

    let card = normalized.parse::<Card>().map_err(AppError::from)?;

    let updated_game = with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameFlowService;
            service
                .play_card(txn, id, seat, card, Some(expected_version.0))
                .await
        })
    })
    .await?;

    let etag = game_etag(updated_game.id, updated_game.lock_version);
    Ok(HttpResponse::NoContent()
        .insert_header((ETAG, etag))
        .finish())
}

/// DELETE /api/games/{game_id}
///
/// Deletes a game. Only the host can delete a game.
///
/// Requires If-Match header with the current game ETag for optimistic locking.
async fn delete_game(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    expected_version: ExpectedVersion,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let host_user_id = membership.user_id;
    let host_turn_order = membership.turn_order;

    with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            // Load game to verify it exists, check host authorization, and validate lock version
            let game = games_sea::require_game(txn, id).await?;

            // Check host authorization
            let is_host = match game.created_by {
                Some(created_by) => created_by == host_user_id,
                None => host_turn_order == 0,
            };

            if !is_host {
                return Err(AppError::forbidden_with_code(
                    ErrorCode::Forbidden,
                    "Only the host can delete a game",
                ));
            }

            // Validate optimistic lock version
            if game.lock_version != expected_version.0 {
                return Err(AppError::conflict(
                    ErrorCode::OptimisticLock,
                    format!(
                        "Game lock version mismatch: expected {}, but game has version {}",
                        expected_version.0, game.lock_version
                    ),
                ));
            }

            // Delete the game with optimistic locking (filter by lock_version)
            // Cascade delete will handle related records automatically
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
            let delete_result = games::Entity::delete_many()
                .filter(games::Column::Id.eq(id))
                .filter(games::Column::LockVersion.eq(expected_version.0))
                .exec(txn)
                .await
                .map_err(|e| AppError::db("failed to delete game", e))?;

            if delete_result.rows_affected == 0 {
                // This should not happen if lock version matched, but handle it gracefully
                // It could happen if the game was deleted by another transaction between
                // the load above and the delete here
                return Err(AppError::conflict(
                    ErrorCode::OptimisticLock,
                    format!(
                        "Game lock version mismatch: expected version {}, but game was modified or deleted",
                        expected_version.0
                    ),
                ));
            }

            Ok(())
        })
    })
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

pub fn friendly_ai_name(user_id: i64, seat_index: usize) -> String {
    const AI_NAMES: [&str; 16] = [
        "Atlas", "Blaze", "Comet", "Dynamo", "Echo", "Flare", "Glyph", "Helix", "Ion", "Jet",
        "Kilo", "Lumen", "Nova", "Orion", "Pulse", "Quark",
    ];

    let idx = ((user_id as usize) ^ seat_index) % AI_NAMES.len();
    AI_NAMES[idx].to_string()
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

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("").route(web::post().to(create_game)));
    cfg.service(web::resource("/joinable").route(web::get().to(list_joinable_games)));
    cfg.service(web::resource("/in-progress").route(web::get().to(list_in_progress_games)));
    cfg.service(web::resource("/last-active").route(web::get().to(get_last_active_game)));
    cfg.service(web::resource("/{game_id}/join").route(web::post().to(join_game)));
    cfg.service(web::resource("/{game_id}/ready").route(web::post().to(mark_ready)));
    cfg.service(web::resource("/ai/registry").route(web::get().to(list_registered_ais)));
    cfg.service(web::resource("/{game_id}/ai/add").route(web::post().to(add_ai_seat)));
    cfg.service(web::resource("/{game_id}/ai/update").route(web::post().to(update_ai_seat)));
    cfg.service(web::resource("/{game_id}/ai/remove").route(web::post().to(remove_ai_seat)));
    cfg.service(web::resource("/{game_id}/bid").route(web::post().to(submit_bid)));
    cfg.service(web::resource("/{game_id}/trump").route(web::post().to(select_trump)));
    cfg.service(web::resource("/{game_id}/play").route(web::post().to(play_card)));
    cfg.service(web::resource("/{game_id}/snapshot").route(web::get().to(get_snapshot)));
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
