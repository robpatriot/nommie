use actix_web::{test, web, HttpMessage, Responder};
use backend::config::db::{DbKind, RuntimeEnv};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::extractors::game_id::GameId;
use backend::infra::state::build_state;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;
use time::OffsetDateTime;

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;

/// Test-only handler that echoes back the game_id for testing
async fn echo(game_id: GameId) -> Result<impl Responder, backend::AppError> {
    #[derive(serde::Serialize)]
    struct Out {
        game_id: i64,
    }
    Ok(web::Json(Out { game_id: game_id.0 }))
}

#[actix_web::test]
async fn happy_path_returns_id() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;

    // Get pooled DB and open a shared txn
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Create a test game in the database using the shared txn
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
    let game_id = game.id;

    // Build test app with echo route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route("/games/{game_id}/echo", web::get().to(echo));
        })
        .build()
        .await?;

    // Make request with existing game id and inject the shared txn
    let req = test::TestRequest::get()
        .uri(&format!("/games/{game_id}/echo"))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Assert success
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["game_id"], game_id);

    // Cleanup — explicitly rollback the shared transaction.
    // This will error if another clone still exists (good safety check).
    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn invalid_id_non_numeric_is_400() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;

    // Build test app with echo route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route("/games/{game_id}/echo", web::get().to(echo));
        })
        .build()
        .await?;

    // Make request with non-numeric game id
    let req = test::TestRequest::get().uri("/games/abc/echo").to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 400
    assert_eq!(resp.status().as_u16(), 400);

    // Validate error structure
    assert_problem_details_structure(resp, 400, "INVALID_GAME_ID", "Invalid game id: abc").await;

    Ok(())
}

#[actix_web::test]
async fn invalid_id_negative_is_400() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;

    // Build test app with echo route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route("/games/{game_id}/echo", web::get().to(echo));
        })
        .build()
        .await?;

    // Make request with negative game id
    let req = test::TestRequest::get().uri("/games/-5/echo").to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 400
    assert_eq!(resp.status().as_u16(), 400);

    // Validate error structure
    assert_problem_details_structure(
        resp,
        400,
        "INVALID_GAME_ID",
        "Game id must be positive, got: -5",
    )
    .await;

    Ok(())
}

#[actix_web::test]
async fn invalid_id_zero_is_400() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;

    // Build test app with echo route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route("/games/{game_id}/echo", web::get().to(echo));
        })
        .build()
        .await?;

    // Make request with zero game id
    let req = test::TestRequest::get().uri("/games/0/echo").to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 400
    assert_eq!(resp.status().as_u16(), 400);

    // Validate error structure
    assert_problem_details_structure(
        resp,
        400,
        "INVALID_GAME_ID",
        "Game id must be positive, got: 0",
    )
    .await;

    Ok(())
}

#[actix_web::test]
async fn not_found_is_404() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;

    // Build test app with echo route
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route("/games/{game_id}/echo", web::get().to(echo));
        })
        .build()
        .await?;

    // Make request with non-existent game id
    let req = test::TestRequest::get()
        .uri("/games/999999/echo")
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 404
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
