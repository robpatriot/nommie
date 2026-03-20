// Integration tests for GameMembership extractor.
//
// Tests the basic behavior of the GameMembership extractor including:
// - Successful extraction when user is a member
// - Error handling for non-members, invalid users, and missing games
// - Composition with CurrentUser and GameId extractors

use actix_web::{test, web, HttpMessage, Responder};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::game_players;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::{CurrentUser, GameId, GameMembership};
use backend_test_support::unique_helpers::{unique_email, unique_str};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;
use time::OffsetDateTime;

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;
use crate::support::factory::create_test_user;
use crate::support::test_middleware::TestSessionInjector;
use crate::support::{test_seed, test_state_builder};

/// Test-only handler that echoes back the membership for testing
async fn echo_membership(
    current_user: CurrentUser,
    game_id: GameId,
    membership: GameMembership,
) -> Result<impl Responder, backend::AppError> {
    #[derive(serde::Serialize)]
    struct Out {
        user_id: String,
        game_id: i64,
        membership_id: i64,
        turn_order: u8,
        is_ready: bool,
        role: String,
    }
    Ok(web::Json(Out {
        user_id: current_user.id.to_string(),
        game_id: game_id.0,
        membership_id: membership.id,
        turn_order: membership.turn_order.unwrap_or(0),
        is_ready: membership.is_ready,
        role: format!("{:?}", membership.role),
    }))
}

#[actix_web::test]
async fn test_membership_success() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(&db).await?;

    let user_sub = unique_str("test-user");
    let user_email = unique_email("test");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("testuser")).await?;

    let now = OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        rules_version: Set("1.0.0".to_string()),
        version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        rng_seed: Set(test_seed("extractor_game_membership").to_vec()),
        ..Default::default()
    };
    let game = game.insert(shared.transaction()).await?;

    let membership = game_players::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(game.id),
        player_kind: Set(backend::entities::game_players::PlayerKind::Human),
        human_user_id: Set(Some(user_id)),
        original_user_id: Set(None),
        ai_profile_id: Set(None),
        turn_order: Set(Some(1i16)),
        is_ready: Set(false),
        role: Set(backend::entities::game_players::GameRole::Player),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let membership = membership.insert(shared.transaction()).await?;

    let session_injector = TestSessionInjector::new(user_id, &user_sub, &user_email);

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/test-games")
                    .route("/{game_id}/membership", web::get().to(echo_membership)),
            );
        })
        .with_session(session_injector)
        .build()
        .await?;

    let req = test::TestRequest::get()
        .uri(&format!("/test-games/{}/membership", game.id))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["user_id"], user_id.to_string());
    assert_eq!(body["game_id"], game.id);
    assert_eq!(body["membership_id"], membership.id);
    assert_eq!(body["turn_order"], 1);
    assert_eq!(body["is_ready"], false);
    assert_eq!(body["role"], "Player");

    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_membership_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    // Get pooled DB and open a shared txn
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(&db).await?;

    // Create a test user
    let user_sub = unique_str("test-user");
    let user_email = unique_email("test");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("testuser")).await?;

    // Create a test game
    let now = OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        rules_version: Set("1.0.0".to_string()),
        version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        rng_seed: Set(test_seed("extractor_game_membership").to_vec()),
        ..Default::default()
    };
    let game = game.insert(shared.transaction()).await?;

    let session_injector = TestSessionInjector::new(user_id, &user_sub, &user_email);

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/test-games")
                    .route("/{game_id}/membership", web::get().to(echo_membership)),
            );
        })
        .with_session(session_injector)
        .build()
        .await?;

    // Make request with valid session but no membership
    let req = test::TestRequest::get()
        .uri(&format!("/test-games/{}/membership", game.id))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Assert 403 Forbidden
    assert_eq!(resp.status().as_u16(), 403);

    // Validate error structure
    assert_problem_details_structure(
        resp,
        403,
        "NOT_A_MEMBER",
        &format!("User is not a member of game {}", game.id),
    )
    .await;

    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_membership_invalid_user_id() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    // Get pooled DB and open a shared txn
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(&db).await?;

    // Create a test game
    let now = OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        rules_version: Set("1.0.0".to_string()),
        version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        rng_seed: Set(test_seed("extractor_game_membership").to_vec()),
        ..Default::default()
    };
    let game = game.insert(shared.transaction()).await?;

    // Inject a session with a non-existent user id
    let session_injector = TestSessionInjector::new(999999999_i64, "nonexistent-sub", "nonexistent@example.com");

    // Build test app with echo route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/test-games")
                    .route("/{game_id}/membership", web::get().to(echo_membership)),
            );
        })
        .with_session(session_injector)
        .build()
        .await?;

    // Make request with session pointing to non-existent user
    let req = test::TestRequest::get()
        .uri(&format!("/test-games/{}/membership", game.id))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Assert 401 Unauthorized (user not found in database)
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure
    assert_problem_details_structure(resp, 401, "UNAUTHORIZED", "Authentication required").await;

    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_membership_composition_with_current_user_and_game_id(
) -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that GameMembership composes properly with CurrentUser and GameId
    let state = test_state_builder()?.build().await?;

    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(&db).await?;

    let user_sub = unique_str("test-user");
    let user_email = unique_email("test");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("testuser")).await?;

    let now = OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        rules_version: Set("1.0.0".to_string()),
        version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        rng_seed: Set(test_seed("extractor_game_membership").to_vec()),
        ..Default::default()
    };
    let game = game.insert(shared.transaction()).await?;

    let membership = game_players::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(game.id),
        player_kind: Set(backend::entities::game_players::PlayerKind::Human),
        human_user_id: Set(Some(user_id)),
        original_user_id: Set(None),
        ai_profile_id: Set(None),
        turn_order: Set(Some(2i16)),
        is_ready: Set(true),
        role: Set(backend::entities::game_players::GameRole::Player),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let membership = membership.insert(shared.transaction()).await?;

    let session_injector = TestSessionInjector::new(user_id, &user_sub, &user_email);

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/test-games")
                    .route("/{game_id}/membership", web::get().to(echo_membership)),
            );
        })
        .with_session(session_injector)
        .build()
        .await?;

    let req = test::TestRequest::get()
        .uri(&format!("/test-games/{}/membership", game.id))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    // Verify all three extractors worked together
    assert_eq!(body["user_id"], user_id.to_string()); // From CurrentUser
    assert_eq!(body["game_id"], game.id); // From GameId
    assert_eq!(body["membership_id"], membership.id); // From GameMembership
    assert_eq!(body["turn_order"], serde_json::json!(2));
    assert_eq!(body["is_ready"], true);
    assert_eq!(body["role"], "Player");

    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_membership_game_not_found() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that GameId extractor catches non-existent games before GameMembership
    let state = test_state_builder()?.build().await?;

    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(&db).await?;

    let user_sub = unique_str("test-user");
    let user_email = unique_email("test");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("testuser")).await?;

    let session_injector = TestSessionInjector::new(user_id, &user_sub, &user_email);

    // Build test app with echo route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/test-games")
                    .route("/{game_id}/membership", web::get().to(echo_membership)),
            );
        })
        .with_session(session_injector)
        .build()
        .await?;

    // Make request with non-existent game ID
    let req = test::TestRequest::get()
        .uri("/test-games/999999/membership")
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Assert 404 Not Found (from GameId extractor)
    assert_eq!(resp.status().as_u16(), 404);

    // Validate error structure
    assert_problem_details_structure(resp, 404, "GAME_NOT_FOUND", "Game 999999 not found").await;

    shared.rollback().await?;

    Ok(())
}
