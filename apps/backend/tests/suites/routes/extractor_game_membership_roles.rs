// Integration tests for GameMembership extractor role-based access control.
//
// Tests role-based authorization patterns using the GameMembership extractor:
// - Player-only actions
// - Actions that allow any member (Player or Spectator)
// - Role hierarchy logic

use actix_web::{test, web, HttpMessage, Responder};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::game_players;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::extractors::{CurrentUser, GameId, GameMembership};
use backend::middleware::jwt_extract::JwtExtract;
use backend::repos::memberships::GameRole;
use backend::state::security_config::SecurityConfig;
use backend::utils::unique::{unique_email, unique_str};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;
use time::OffsetDateTime;

use crate::support::app_builder::create_test_app;
use crate::support::auth::mint_test_token;
use crate::support::factory::create_test_user;
use crate::support::test_state_builder;

/// Test-only handler that requires Player role (rejects Spectators)
async fn player_only_action(
    current_user: CurrentUser,
    game_id: GameId,
    membership: GameMembership,
) -> Result<impl Responder, backend::AppError> {
    use backend::errors::ErrorCode;

    // Enforce Player role
    if membership.role != GameRole::Player {
        return Err(backend::AppError::forbidden_with_code(
            ErrorCode::InsufficientRole,
            "This action requires Player role",
        ));
    }

    #[derive(serde::Serialize)]
    struct Out {
        user_id: String,
        game_id: i64,
        role: String,
        message: String,
    }
    Ok(web::Json(Out {
        user_id: current_user.sub,
        game_id: game_id.0,
        role: format!("{:?}", membership.role),
        message: "Player action successful".to_string(),
    }))
}

/// Test-only handler that accepts any member (Player or Spectator)
async fn spectator_allowed_action(
    current_user: CurrentUser,
    game_id: GameId,
    membership: GameMembership,
) -> Result<impl Responder, backend::AppError> {
    #[derive(serde::Serialize)]
    struct Out {
        user_id: String,
        game_id: i64,
        role: String,
        message: String,
    }
    Ok(web::Json(Out {
        user_id: current_user.sub,
        game_id: game_id.0,
        role: format!("{:?}", membership.role),
        message: "Any member can view this".to_string(),
    }))
}

#[actix_web::test]
async fn test_role_based_access_player_only() -> Result<(), Box<dyn std::error::Error>> {
    // Test that handlers can enforce role-based access control
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config.clone())
        .build()
        .await?;

    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Create a test user
    let user_sub = unique_str("test-user");
    let user_email = unique_email("test");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("player1")).await?;

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

    // Create a Player membership (currently all memberships are Players)
    let membership = game_players::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(game.id),
        user_id: Set(user_id),
        turn_order: Set(1),
        is_ready: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };
    membership.insert(shared.transaction()).await?;

    // Create a valid JWT token
    let token = mint_test_token(&user_sub, &user_email, &security_config);

    // Build test app with player-only route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(web::scope("/test-games").wrap(JwtExtract).route(
                "/{game_id}/player-action",
                web::post().to(player_only_action),
            ));
        })
        .build()
        .await?;

    // Make request - should succeed because user is a Player
    let req = test::TestRequest::post()
        .uri(&format!("/test-games/{}/player-action", game.id))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["role"], "Player");
    assert_eq!(body["message"], "Player action successful");

    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_role_based_access_any_member() -> Result<(), Box<dyn std::error::Error>> {
    // Test that handlers accepting any role work correctly
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config.clone())
        .build()
        .await?;

    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Create a test user
    let user_sub = unique_str("test-user");
    let user_email = unique_email("test");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("viewer")).await?;

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

    // Create membership
    let membership = game_players::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(game.id),
        user_id: Set(user_id),
        turn_order: Set(1),
        is_ready: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };
    membership.insert(shared.transaction()).await?;

    // Create a valid JWT token
    let token = mint_test_token(&user_sub, &user_email, &security_config);

    // Build test app with spectator-allowed route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/test-games")
                    .wrap(JwtExtract)
                    .route("/{game_id}/view", web::get().to(spectator_allowed_action)),
            );
        })
        .build()
        .await?;

    // Make request - should succeed for any member
    let req = test::TestRequest::get()
        .uri(&format!("/test-games/{}/view", game.id))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["role"], "Player");
    assert_eq!(body["message"], "Any member can view this");

    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_game_role_hierarchy() {
    // Unit test for GameRole::has_at_least logic
    // This demonstrates the role hierarchy system

    // Player has at least Player permission
    assert!(GameRole::Player.has_at_least(GameRole::Player));

    // Player has at least Spectator permission (Players can do everything Spectators can)
    assert!(GameRole::Player.has_at_least(GameRole::Spectator));

    // Spectator does NOT have at least Player permission
    assert!(!GameRole::Spectator.has_at_least(GameRole::Player));

    // Spectator has at least Spectator permission
    assert!(GameRole::Spectator.has_at_least(GameRole::Spectator));
}
