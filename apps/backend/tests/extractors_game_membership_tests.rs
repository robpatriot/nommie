mod common;
mod support;

use actix_web::{test, web, HttpMessage, Responder};
use backend::auth::jwt::mint_access_token;
use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::entities::{game_players, users};
use backend::extractors::{CurrentUser, GameId, GameMembership};
use backend::infra::state::build_state;
use backend::state::security_config::SecurityConfig;
use backend::utils::unique::{unique_email, unique_str};
use common::assert_problem_details_structure;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;
use support::create_test_app;
use time::OffsetDateTime;

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
        turn_order: i32,
        is_ready: bool,
        role: String,
    }
    Ok(web::Json(Out {
        user_id: current_user.sub,
        game_id: game_id.0,
        membership_id: membership.id,
        turn_order: membership.turn_order,
        is_ready: membership.is_ready,
        role: format!("{:?}", membership.role),
    }))
}

/// Test-only handler that requires specific role
#[allow(dead_code)]
async fn echo_membership_with_role(
    current_user: CurrentUser,
    game_id: GameId,
    membership: GameMembership,
) -> Result<impl Responder, backend::AppError> {
    // This handler would use RoleGuard::Exactly(GameRole::Player) in real usage
    #[derive(serde::Serialize)]
    struct Out {
        user_id: String,
        game_id: i64,
        role: String,
    }
    Ok(web::Json(Out {
        user_id: current_user.sub,
        game_id: game_id.0,
        role: format!("{:?}", membership.role),
    }))
}

#[actix_web::test]
async fn test_membership_success() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config.clone())
        .build()
        .await?;

    // Get pooled DB and open a shared txn
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Create a test user
    let user_sub = unique_str("test-user");
    let user_email = unique_email("test");
    let now = OffsetDateTime::now_utc();
    let user = users::ActiveModel {
        id: sea_orm::NotSet,
        sub: Set(user_sub.clone()),
        username: Set(Some("testuser".to_string())),
        is_ai: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let user = user.insert(shared.transaction()).await?;

    // Create a test game
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        rules_version: Set("1.0.0".to_string()),
        lock_version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let game = game.insert(shared.transaction()).await?;

    // Create a game membership
    let membership = game_players::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(game.id),
        user_id: Set(user.id),
        turn_order: Set(1),
        is_ready: Set(false),
        created_at: Set(now),
    };
    let membership = membership.insert(shared.transaction()).await?;

    // Create a valid JWT token
    let token = mint_access_token(
        &user_sub,
        &user_email,
        std::time::SystemTime::now(),
        &security_config,
    )
    .unwrap();

    // Build test app with echo route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route(
                "/games/{game_id}/membership",
                web::get().to(echo_membership),
            );
        })
        .build()
        .await?;

    // Make request with valid token and inject the shared txn
    let req = test::TestRequest::get()
        .uri(&format!("/games/{}/membership", game.id))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Assert success
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["user_id"], user_sub);
    assert_eq!(body["game_id"], game.id);
    assert_eq!(body["membership_id"], membership.id);
    assert_eq!(body["turn_order"], 1);
    assert_eq!(body["is_ready"], false);
    assert_eq!(body["role"], "Player");

    // Cleanup
    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_membership_not_found() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config.clone())
        .build()
        .await?;

    // Get pooled DB and open a shared txn
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Create a test user
    let user_sub = unique_str("test-user");
    let user_email = unique_email("test");
    let now = OffsetDateTime::now_utc();
    let user = users::ActiveModel {
        id: sea_orm::NotSet,
        sub: Set(user_sub.clone()),
        username: Set(Some("testuser".to_string())),
        is_ai: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let _user = user.insert(shared.transaction()).await?;

    // Create a test game
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        rules_version: Set("1.0.0".to_string()),
        lock_version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let game = game.insert(shared.transaction()).await?;

    // Create a valid JWT token
    let token = mint_access_token(
        &user_sub,
        &user_email,
        std::time::SystemTime::now(),
        &security_config,
    )
    .unwrap();

    // Build test app with echo route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route(
                "/games/{game_id}/membership",
                web::get().to(echo_membership),
            );
        })
        .build()
        .await?;

    // Make request with valid token but no membership
    let req = test::TestRequest::get()
        .uri(&format!("/games/{}/membership", game.id))
        .insert_header(("Authorization", format!("Bearer {token}")))
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

    // Cleanup
    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_membership_invalid_user_id() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config.clone())
        .build()
        .await?;

    // Get pooled DB and open a shared txn
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Create a test game
    let now = OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        rules_version: Set("1.0.0".to_string()),
        lock_version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let game = game.insert(shared.transaction()).await?;

    // Create a JWT token with invalid user sub (non-numeric)
    let user_sub = "invalid-user-id";
    let user_email = unique_email("test");
    let token = mint_access_token(
        user_sub,
        &user_email,
        std::time::SystemTime::now(),
        &security_config,
    )
    .unwrap();

    // Build test app with echo route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route(
                "/games/{game_id}/membership",
                web::get().to(echo_membership),
            );
        })
        .build()
        .await?;

    // Make request with invalid user ID
    let req = test::TestRequest::get()
        .uri(&format!("/games/{}/membership", game.id))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Assert 403 Forbidden (user not found)
    assert_eq!(resp.status().as_u16(), 403);

    // Validate error structure
    assert_problem_details_structure(
        resp,
        403,
        "FORBIDDEN_USER_NOT_FOUND",
        "User not found in database",
    )
    .await;

    // Cleanup
    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_membership_composition_with_current_user_and_game_id(
) -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that GameMembership composes properly with CurrentUser and GameId
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config.clone())
        .build()
        .await?;

    // Get pooled DB and open a shared txn
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Create a test user
    let user_sub = unique_str("test-user");
    let user_email = unique_email("test");
    let now = OffsetDateTime::now_utc();
    let user = users::ActiveModel {
        id: sea_orm::NotSet,
        sub: Set(user_sub.clone()),
        username: Set(Some("testuser".to_string())),
        is_ai: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let user = user.insert(shared.transaction()).await?;

    // Create a test game
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        rules_version: Set("1.0.0".to_string()),
        lock_version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let game = game.insert(shared.transaction()).await?;

    // Create a game membership
    let membership = game_players::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(game.id),
        user_id: Set(user.id),
        turn_order: Set(2),
        is_ready: Set(true),
        created_at: Set(now),
    };
    let membership = membership.insert(shared.transaction()).await?;

    // Create a valid JWT token
    let token = mint_access_token(
        &user_sub,
        &user_email,
        std::time::SystemTime::now(),
        &security_config,
    )
    .unwrap();

    // Build test app with echo route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route(
                "/games/{game_id}/membership",
                web::get().to(echo_membership),
            );
        })
        .build()
        .await?;

    // Make request with valid token and inject the shared txn
    let req = test::TestRequest::get()
        .uri(&format!("/games/{}/membership", game.id))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Assert success
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    // Verify all three extractors worked together
    assert_eq!(body["user_id"], user_sub); // From CurrentUser
    assert_eq!(body["game_id"], game.id); // From GameId
    assert_eq!(body["membership_id"], membership.id); // From GameMembership
    assert_eq!(body["turn_order"], 2);
    assert_eq!(body["is_ready"], true);
    assert_eq!(body["role"], "Player");

    // Cleanup
    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_membership_game_not_found() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that GameId extractor catches non-existent games before GameMembership
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config.clone())
        .build()
        .await?;

    // Create a JWT token
    let user_sub = unique_str("test-user");
    let user_email = unique_email("test");
    let token = mint_access_token(
        &user_sub,
        &user_email,
        std::time::SystemTime::now(),
        &security_config,
    )
    .unwrap();

    // Build test app with echo route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route(
                "/games/{game_id}/membership",
                web::get().to(echo_membership),
            );
        })
        .build()
        .await?;

    // Make request with non-existent game ID
    let req = test::TestRequest::get()
        .uri("/games/999999/membership")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 404 Not Found (from GameId extractor)
    assert_eq!(resp.status().as_u16(), 404);

    // Validate error structure
    assert_problem_details_structure(
        resp,
        404,
        "GAME_NOT_FOUND",
        "Game not found with id: 999999",
    )
    .await;

    Ok(())
}
