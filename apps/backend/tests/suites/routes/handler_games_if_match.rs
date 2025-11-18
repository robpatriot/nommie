// HTTP-level tests for If-Match header support on mutation endpoints.
//
// Tests include:
// - POST /api/games/{id}/bid with matching If-Match succeeds and bumps ETag
// - POST /api/games/{id}/bid with stale If-Match returns 409 Conflict
// - POST /api/games/{id}/bid missing If-Match returns 428 Precondition Required
// - POST /api/games/{id}/trump with matching If-Match succeeds and bumps ETag
// - POST /api/games/{id}/trump with stale If-Match returns 409 Conflict
// - POST /api/games/{id}/trump missing If-Match returns 428 Precondition Required
// - POST /api/games/{id}/play with matching If-Match succeeds and bumps ETag
// - POST /api/games/{id}/play with stale If-Match returns 409 Conflict
// - POST /api/games/{id}/play missing If-Match returns 428 Precondition Required
// - DELETE /api/games/{id} with matching If-Match succeeds
// - DELETE /api/games/{id} with stale If-Match returns 409 Conflict
// - DELETE /api/games/{id} missing If-Match returns 428 Precondition Required

use actix_web::http::header::{HeaderValue, IF_MATCH};
use actix_web::http::StatusCode;
use actix_web::{test, web, HttpMessage};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::games;
use backend::error::AppError;
use backend::http::etag::game_etag;
use backend::middleware::jwt_extract::JwtExtract;
use backend::routes::games as games_routes;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};

use crate::support::app_builder::create_test_app;
use crate::support::auth::mint_test_token;
use crate::support::build_test_state;
use crate::support::db_memberships::create_test_game_player_with_ready;
use crate::support::factory::{create_fresh_lobby_game, create_test_user};
use crate::support::game_phases::{
    setup_game_in_bidding_phase, setup_game_in_trick_play_phase,
    setup_game_in_trump_selection_phase,
};

struct IfMatchTestContext {
    state: backend::state::app_state::AppState,
    shared: SharedTxn,
    bearer: String,
    game_id: i64,
}

async fn setup_bidding_test(test_name: &str) -> Result<IfMatchTestContext, AppError> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = SharedTxn::open(db).await?;

    let setup = setup_game_in_bidding_phase(shared.transaction(), test_name).await?;
    let dealer_pos = setup.dealer_pos as usize;
    let actor_seat = ((dealer_pos + 1) % 4) as i16;

    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    // Update membership to use our test user
    use backend::entities::game_players;
    let membership = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(setup.game_id))
        .filter(game_players::Column::TurnOrder.eq(actor_seat))
        .one(shared.transaction())
        .await?
        .expect("membership should exist");

    let mut membership: game_players::ActiveModel = membership.into();
    membership.human_user_id = sea_orm::Set(Some(user_id));
    ActiveModelTrait::update(membership, shared.transaction()).await?;

    let token = mint_test_token(&user_sub, &user_email, &security);

    Ok(IfMatchTestContext {
        state,
        shared,
        bearer: format!("Bearer {token}"),
        game_id: setup.game_id,
    })
}

async fn setup_trump_test(test_name: &str) -> Result<IfMatchTestContext, AppError> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = SharedTxn::open(db).await?;

    let setup =
        setup_game_in_trump_selection_phase(shared.transaction(), test_name, [3, 3, 4, 2]).await?;

    // Find winning bidder (highest bid, ties go to earliest)
    let winning_bidder = 2; // Seat 2 has bid 4

    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    // Update membership to use our test user
    use backend::entities::game_players;
    let membership = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(setup.game_id))
        .filter(game_players::Column::TurnOrder.eq(winning_bidder))
        .one(shared.transaction())
        .await?
        .expect("membership should exist");

    let mut membership: game_players::ActiveModel = membership.into();
    membership.human_user_id = sea_orm::Set(Some(user_id));
    ActiveModelTrait::update(membership, shared.transaction()).await?;

    let token = mint_test_token(&user_sub, &user_email, &security);

    Ok(IfMatchTestContext {
        state,
        shared,
        bearer: format!("Bearer {token}"),
        game_id: setup.game_id,
    })
}

async fn setup_play_test(
    test_name: &str,
) -> Result<(IfMatchTestContext, backend::domain::Card), AppError> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = SharedTxn::open(db).await?;

    let setup = setup_game_in_trick_play_phase(
        shared.transaction(),
        test_name,
        [3, 3, 4, 2],
        backend::repos::rounds::Trump::Hearts,
    )
    .await?;

    // First player to play in trick
    let first_player = 0i32;

    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    // Update membership to use our test user
    use backend::entities::game_players;
    let membership = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(setup.game_id))
        .filter(game_players::Column::TurnOrder.eq(first_player))
        .one(shared.transaction())
        .await?
        .expect("membership should exist");

    let mut membership: game_players::ActiveModel = membership.into();
    membership.human_user_id = sea_orm::Set(Some(user_id));
    ActiveModelTrait::update(membership, shared.transaction()).await?;

    // Get a card from the player's hand directly
    use backend::repos::hands;
    let game =
        backend::adapters::games_sea::require_game(shared.transaction(), setup.game_id).await?;
    let round = backend::repos::rounds::find_by_game_and_round(
        shared.transaction(),
        setup.game_id,
        game.current_round.expect("game should have current round"),
    )
    .await?
    .expect("round should exist");
    let hand = hands::find_by_round_and_seat(shared.transaction(), round.id, first_player as i16)
        .await?
        .expect("player should have a hand");
    let first_card = backend::domain::cards_parsing::from_stored_format(
        &hand.cards[0].suit,
        &hand.cards[0].rank,
    )?;
    let card_to_play = first_card;

    let token = mint_test_token(&user_sub, &user_email, &security);

    let ctx = IfMatchTestContext {
        state,
        shared,
        bearer: format!("Bearer {token}"),
        game_id: setup.game_id,
    };

    Ok((ctx, card_to_play))
}

async fn setup_delete_test(test_name: &str) -> Result<IfMatchTestContext, AppError> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = SharedTxn::open(db).await?;

    let game_id = create_fresh_lobby_game(shared.transaction(), test_name).await?;
    let user_sub = format!("{test_name}_host");
    let user_email = format!("{test_name}@example.com");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Host User")).await?;

    // Create membership as host (turn_order 0)
    create_test_game_player_with_ready(shared.transaction(), game_id, user_id, 0, false).await?;

    // Set created_by to make user the host
    let game = games::Entity::find_by_id(game_id)
        .one(shared.transaction())
        .await?
        .expect("game should exist");
    let mut game: games::ActiveModel = game.into();
    game.created_by = sea_orm::Set(Some(user_id));
    ActiveModelTrait::update(game, shared.transaction()).await?;

    let token = mint_test_token(&user_sub, &user_email, &security);

    Ok(IfMatchTestContext {
        state,
        shared,
        bearer: format!("Bearer {token}"),
        game_id,
    })
}

fn get_current_etag(game_id: i64, lock_version: i32) -> String {
    game_etag(game_id, lock_version)
}

#[tokio::test]
async fn test_bid_with_matching_if_match_succeeds_and_bumps_etag() -> Result<(), AppError> {
    let ctx = setup_bidding_test("bid_matching_if_match").await?;

    // Get initial ETag
    let game = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?
        .expect("game should exist");
    let initial_etag = get_current_etag(ctx.game_id, game.lock_version);

    let app = create_test_app(ctx.state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games_routes::configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::post()
        .uri(&format!("/api/games/{}/bid", ctx.game_id))
        .insert_header(("Authorization", ctx.bearer.clone()))
        .insert_header((IF_MATCH, HeaderValue::from_str(&initial_etag).unwrap()))
        .set_json(serde_json::json!({ "bid": 3 }))
        .to_request();
    req.extensions_mut().insert(ctx.shared.clone());

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify ETag was bumped - extract header before consuming body
    let etag_header = resp
        .headers()
        .get("etag")
        .expect("ETag header should be present")
        .to_str()
        .expect("ETag should be valid ASCII string")
        .to_string();

    // Consume response body
    let _body = test::read_body(resp).await;

    let game_after = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?
        .expect("game should exist");
    let expected_etag = get_current_etag(ctx.game_id, game_after.lock_version);

    assert_eq!(etag_header, expected_etag);
    assert!(
        game_after.lock_version > game.lock_version,
        "lock_version should increment after operation"
    );

    drop(app);
    ctx.shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_bid_with_stale_if_match_returns_409() -> Result<(), AppError> {
    let ctx = setup_bidding_test("bid_stale_if_match").await?;

    // Get initial ETag
    let game = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?
        .expect("game should exist");
    let stale_etag = get_current_etag(ctx.game_id, game.lock_version - 1); // Stale version

    let app = create_test_app(ctx.state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games_routes::configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::post()
        .uri(&format!("/api/games/{}/bid", ctx.game_id))
        .insert_header(("Authorization", ctx.bearer.clone()))
        .insert_header((IF_MATCH, HeaderValue::from_str(&stale_etag).unwrap()))
        .set_json(serde_json::json!({ "bid": 3 }))
        .to_request();
    req.extensions_mut().insert(ctx.shared.clone());

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "OPTIMISTIC_LOCK");

    drop(app);
    ctx.shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_bid_missing_if_match_returns_428() -> Result<(), AppError> {
    let ctx = setup_bidding_test("bid_missing_if_match").await?;

    let app = create_test_app(ctx.state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games_routes::configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::post()
        .uri(&format!("/api/games/{}/bid", ctx.game_id))
        .insert_header(("Authorization", ctx.bearer.clone()))
        // No If-Match header
        .set_json(serde_json::json!({ "bid": 3 }))
        .to_request();
    req.extensions_mut().insert(ctx.shared.clone());

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::PRECONDITION_REQUIRED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "PRECONDITION_REQUIRED");

    ctx.shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_trump_with_matching_if_match_succeeds_and_bumps_etag() -> Result<(), AppError> {
    let ctx = setup_trump_test("trump_matching_if_match").await?;

    // Get initial ETag
    let game = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?
        .expect("game should exist");
    let initial_etag = get_current_etag(ctx.game_id, game.lock_version);

    let app = create_test_app(ctx.state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games_routes::configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::post()
        .uri(&format!("/api/games/{}/trump", ctx.game_id))
        .insert_header(("Authorization", ctx.bearer.clone()))
        .insert_header((IF_MATCH, HeaderValue::from_str(&initial_etag).unwrap()))
        .set_json(serde_json::json!({ "trump": "HEARTS" }))
        .to_request();
    req.extensions_mut().insert(ctx.shared.clone());

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify ETag was bumped - extract header before consuming body
    let etag_header = resp
        .headers()
        .get("etag")
        .expect("ETag header should be present")
        .to_str()
        .expect("ETag should be valid ASCII string")
        .to_string();

    // Consume response body
    let _body = test::read_body(resp).await;

    let game_after = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?
        .expect("game should exist");
    let expected_etag = get_current_etag(ctx.game_id, game_after.lock_version);

    assert_eq!(etag_header, expected_etag);
    assert!(
        game_after.lock_version > game.lock_version,
        "lock_version should increment after operation"
    );

    drop(app);
    ctx.shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_trump_with_stale_if_match_returns_409() -> Result<(), AppError> {
    let ctx = setup_trump_test("trump_stale_if_match").await?;

    // Get initial ETag
    let game = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?
        .expect("game should exist");
    let stale_etag = get_current_etag(ctx.game_id, game.lock_version - 1); // Stale version

    let app = create_test_app(ctx.state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games_routes::configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::post()
        .uri(&format!("/api/games/{}/trump", ctx.game_id))
        .insert_header(("Authorization", ctx.bearer.clone()))
        .insert_header((IF_MATCH, HeaderValue::from_str(&stale_etag).unwrap()))
        .set_json(serde_json::json!({ "trump": "HEARTS" }))
        .to_request();
    req.extensions_mut().insert(ctx.shared.clone());

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "OPTIMISTIC_LOCK");

    drop(app);
    ctx.shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_play_with_matching_if_match_succeeds_and_bumps_etag() -> Result<(), AppError> {
    let (ctx, card_to_play) = setup_play_test("play_matching_if_match").await?;

    // Format card as string (e.g., "AH" for Ace of Hearts)
    use backend::domain::{Rank, Suit};
    let rank_char = match card_to_play.rank {
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
    let suit_char = match card_to_play.suit {
        Suit::Clubs => 'C',
        Suit::Diamonds => 'D',
        Suit::Hearts => 'H',
        Suit::Spades => 'S',
    };
    let card_str = format!("{rank_char}{suit_char}");

    // Get initial ETag
    let game = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?
        .expect("game should exist");
    let initial_etag = get_current_etag(ctx.game_id, game.lock_version);

    let app = create_test_app(ctx.state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games_routes::configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::post()
        .uri(&format!("/api/games/{}/play", ctx.game_id))
        .insert_header(("Authorization", ctx.bearer.clone()))
        .insert_header((IF_MATCH, HeaderValue::from_str(&initial_etag).unwrap()))
        .set_json(serde_json::json!({ "card": card_str }))
        .to_request();
    req.extensions_mut().insert(ctx.shared.clone());

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify ETag was bumped - extract header before consuming body
    let etag_header = resp
        .headers()
        .get("etag")
        .expect("ETag header should be present")
        .to_str()
        .expect("ETag should be valid ASCII string")
        .to_string();

    // Consume response body
    let _body = test::read_body(resp).await;

    let game_after = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?
        .expect("game should exist");
    let expected_etag = get_current_etag(ctx.game_id, game_after.lock_version);

    assert_eq!(etag_header, expected_etag);
    assert!(
        game_after.lock_version > game.lock_version,
        "lock_version should increment after operation"
    );

    drop(app);
    ctx.shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_play_with_stale_if_match_returns_409() -> Result<(), AppError> {
    let (ctx, card_to_play) = setup_play_test("play_stale_if_match").await?;

    // Format card as string
    use backend::domain::{Rank, Suit};
    let rank_char = match card_to_play.rank {
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
    let suit_char = match card_to_play.suit {
        Suit::Clubs => 'C',
        Suit::Diamonds => 'D',
        Suit::Hearts => 'H',
        Suit::Spades => 'S',
    };
    let card_str = format!("{rank_char}{suit_char}");

    // Get initial ETag
    let game = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?
        .expect("game should exist");
    let stale_etag = get_current_etag(ctx.game_id, game.lock_version - 1); // Stale version

    let app = create_test_app(ctx.state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games_routes::configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::post()
        .uri(&format!("/api/games/{}/play", ctx.game_id))
        .insert_header(("Authorization", ctx.bearer.clone()))
        .insert_header((IF_MATCH, HeaderValue::from_str(&stale_etag).unwrap()))
        .set_json(serde_json::json!({ "card": card_str }))
        .to_request();
    req.extensions_mut().insert(ctx.shared.clone());

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "OPTIMISTIC_LOCK");

    drop(app);
    ctx.shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_delete_with_matching_if_match_succeeds() -> Result<(), AppError> {
    let ctx = setup_delete_test("delete_matching_if_match").await?;

    // Get initial ETag
    let game = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?
        .expect("game should exist");
    let initial_etag = get_current_etag(ctx.game_id, game.lock_version);

    let app = create_test_app(ctx.state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games_routes::configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/games/{}", ctx.game_id))
        .insert_header(("Authorization", ctx.bearer.clone()))
        .insert_header((IF_MATCH, HeaderValue::from_str(&initial_etag).unwrap()))
        .to_request();
    req.extensions_mut().insert(ctx.shared.clone());

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify game was deleted
    drop(resp);
    let game_after = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?;
    assert!(game_after.is_none());

    ctx.shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_delete_with_stale_if_match_returns_409() -> Result<(), AppError> {
    let ctx = setup_delete_test("delete_stale_if_match").await?;

    // Get initial ETag
    let game = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?
        .expect("game should exist");
    let stale_etag = get_current_etag(ctx.game_id, game.lock_version - 1); // Stale version

    let app = create_test_app(ctx.state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games_routes::configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/games/{}", ctx.game_id))
        .insert_header(("Authorization", ctx.bearer.clone()))
        .insert_header((IF_MATCH, HeaderValue::from_str(&stale_etag).unwrap()))
        .to_request();
    req.extensions_mut().insert(ctx.shared.clone());

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "OPTIMISTIC_LOCK");

    // Verify game was NOT deleted
    let _game_after = games::Entity::find_by_id(ctx.game_id)
        .one(ctx.shared.transaction())
        .await?
        .expect("game should still exist");

    ctx.shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_delete_missing_if_match_returns_428() -> Result<(), AppError> {
    let ctx = setup_delete_test("delete_missing_if_match").await?;

    let app = create_test_app(ctx.state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games_routes::configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/games/{}", ctx.game_id))
        .insert_header(("Authorization", ctx.bearer.clone()))
        // No If-Match header
        .to_request();
    req.extensions_mut().insert(ctx.shared.clone());

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::PRECONDITION_REQUIRED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "PRECONDITION_REQUIRED");

    ctx.shared.rollback().await?;
    Ok(())
}
