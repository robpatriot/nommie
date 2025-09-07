mod common;

use actix_web::{test, web, App, Responder};
use backend::config::db::DbProfile;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::extractors::game_id::GameId;
use backend::infra::state::build_state;
use backend::middleware::request_trace::RequestTrace;
use common::assert_problem_details_structure;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;
use time::OffsetDateTime;

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
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Create a test game in the database
    let db = state.db.as_ref().unwrap();
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
    let game = game.insert(db).await?;
    let game_id = game.id;

    // Build test app with echo route
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .wrap(RequestTrace)
            .route("/games/{game_id}/echo", web::get().to(echo)),
    )
    .await;

    // Make request with existing game id
    let req = test::TestRequest::get()
        .uri(&format!("/games/{game_id}/echo"))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert success
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["game_id"], game_id);

    Ok(())
}

#[actix_web::test]
async fn invalid_id_non_numeric_is_400() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Build test app with echo route
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .wrap(RequestTrace)
            .route("/games/{game_id}/echo", web::get().to(echo)),
    )
    .await;

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
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Build test app with echo route
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .wrap(RequestTrace)
            .route("/games/{game_id}/echo", web::get().to(echo)),
    )
    .await;

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
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Build test app with echo route
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .wrap(RequestTrace)
            .route("/games/{game_id}/echo", web::get().to(echo)),
    )
    .await;

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
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Build test app with echo route
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .wrap(RequestTrace)
            .route("/games/{game_id}/echo", web::get().to(echo)),
    )
    .await;

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
