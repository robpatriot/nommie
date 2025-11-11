//! Game-related HTTP routes.

use actix_web::http::header::{ETAG, IF_NONE_MATCH};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use rand::random;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::warn;
use uuid::Uuid;

use crate::adapters::games_sea::{self, GameUpdateRound};
use crate::db::txn::with_txn;
use crate::domain::player_view::CurrentRoundInfo;
use crate::domain::snapshot::{GameSnapshot, SeatPublic};
use crate::domain::state::Seat;
use crate::domain::{Card, Rank, Suit};
use crate::entities::games::{GameState, GameVisibility};
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::extractors::current_user::CurrentUser;
use crate::extractors::game_id::GameId;
use crate::extractors::game_membership::GameMembership;
use crate::extractors::ValidatedJson;
use crate::http::etag::game_etag;
use crate::repos::memberships::{self, GameRole};
use crate::repos::{ai_overrides, users};
use crate::services::ai::AiService;
use crate::services::game_flow::GameFlowService;
use crate::services::games::GameService;
use crate::services::players::PlayerService;
use crate::state::app_state::AppState;

#[derive(Serialize)]
struct GameSnapshotResponse {
    snapshot: GameSnapshot,
    viewer_hand: Option<Vec<String>>,
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
    let (snapshot, lock_version, viewer_seat, viewer_hand) =
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
                            if let Some(username) = &user_ref.username {
                                let trimmed = username.trim();
                                let looks_machine =
                                    user_ref.is_ai && looks_like_machine_identifier(trimmed);
                                if !trimmed.is_empty() && !looks_machine {
                                    display_name = Some(trimmed.to_owned());
                                }
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

                Ok((snap, game.lock_version, viewer_seat, viewer_hand))
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
) -> GameResponse {
    // Count players (exclude spectators)
    let player_count = memberships
        .iter()
        .filter(|m| m.role == GameRole::Player)
        .count() as i32;

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

    let response = game_to_response(&game_model, &memberships);

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

    let response = game_to_response(&game_model, &memberships);

    Ok(web::Json(JoinGameResponse { game: response }))
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
struct PlayCardRequest {
    card: String,
}

#[derive(Debug, Default, Deserialize)]
struct ManageAiSeatRequest {
    seat: Option<u8>,
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
            let ai_service = AiService;
            let suffix = &Uuid::new_v4().to_string()[..8];
            let ai_display_name = format!("Bot {} {suffix}", seat_index + 1);
            let ai_config = Some(json!({ "seed": random::<u64>() }));

            let ai_user_id = ai_service
                .create_ai_template_user(txn, ai_display_name, "random", ai_config, Some(100))
                .await
                .map_err(AppError::from)?;

            ai_service
                .add_ai_to_game(txn, id, ai_user_id, seat_to_fill, None)
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

/// POST /api/games/{game_id}/bid
///
/// Submits a bid for the current player. Bidding order and validation are enforced
/// by the service layer.
async fn submit_bid(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: ValidatedJson<SubmitBidRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id = game_id.0;
    let seat = membership.turn_order as i16;
    let bid_value = body.bid;

    with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameFlowService;
            service.submit_bid(txn, id, seat, bid_value).await?;
            Ok(())
        })
    })
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

async fn play_card(
    http_req: HttpRequest,
    game_id: GameId,
    membership: GameMembership,
    body: ValidatedJson<PlayCardRequest>,
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

    with_txn(Some(&http_req), &app_state, |txn| {
        Box::pin(async move {
            let service = GameFlowService;
            service.play_card(txn, id, seat, card).await?;
            Ok(())
        })
    })
    .await?;

    Ok(HttpResponse::NoContent().finish())
}

fn looks_like_machine_identifier(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.starts_with("ai:") {
        return true;
    }

    trimmed.len() >= 16
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() || matches!(ch, '-' | ':'))
}

fn friendly_ai_name(user_id: i64, seat_index: usize) -> String {
    const AI_NAMES: [&str; 16] = [
        "Atlas", "Blaze", "Comet", "Dynamo", "Echo", "Flare", "Glyph", "Helix", "Ion", "Jet",
        "Kilo", "Lumen", "Nova", "Orion", "Pulse", "Quark",
    ];

    let idx = ((user_id as usize) ^ seat_index) % AI_NAMES.len();
    AI_NAMES[idx].to_string()
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
    cfg.service(web::resource("/{game_id}/join").route(web::post().to(join_game)));
    cfg.service(web::resource("/{game_id}/ready").route(web::post().to(mark_ready)));
    cfg.service(web::resource("/{game_id}/ai/add").route(web::post().to(add_ai_seat)));
    cfg.service(web::resource("/{game_id}/ai/remove").route(web::post().to(remove_ai_seat)));
    cfg.service(web::resource("/{game_id}/bid").route(web::post().to(submit_bid)));
    cfg.service(web::resource("/{game_id}/play").route(web::post().to(play_card)));
    cfg.service(web::resource("/{game_id}/snapshot").route(web::get().to(get_snapshot)));
    cfg.service(
        web::resource("/{game_id}/players/{seat}/display_name")
            .route(web::get().to(get_player_display_name)),
    );
}
