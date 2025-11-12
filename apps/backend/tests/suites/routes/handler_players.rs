use actix_web::http::StatusCode;
use actix_web::{test, web, HttpMessage};
use backend::ai::RandomPlayer;
use backend::error::AppError;
use backend::repos::ai_profiles;
use backend::routes::games::configure_routes;
use backend::services::ai::AiService;

use crate::support::app_builder::create_test_app;
use crate::support::build_test_state;
use crate::support::db_memberships::create_test_game_player;
use crate::support::factory::{create_test_game, create_test_user};

#[tokio::test]
async fn test_get_player_display_name_success() -> Result<(), AppError> {
    let state = build_test_state().await?;

    // Get pooled DB and open a shared txn
    let db = backend::db::require_db(&state).expect("DB required for this test");
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    // Create test data using the shared transaction
    let game_id = create_test_game(shared.transaction()).await?;
    let user_id = create_test_user(shared.transaction(), "alice", Some("AliceUser")).await?;
    create_test_game_player(shared.transaction(), game_id, user_id, 0).await?;

    // Create test app
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(web::scope("/api/games").configure(configure_routes));
        })
        .build()
        .await?;

    // Test the endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/players/0/display_name"))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Parse response body
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["display_name"], "AliceUser");

    // Rollback the transaction after all assertions complete
    // (Reading the response body fully should have dropped request extensions)
    shared.rollback().await?;

    Ok(())
}

#[tokio::test]
async fn test_get_player_display_name_ai_user() -> Result<(), AppError> {
    let state = build_test_state().await?;

    // Get pooled DB and open a shared txn
    let db = backend::db::require_db(&state).expect("DB required for this test");
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    // Create test data with AI user
    let game_id = create_test_game(shared.transaction()).await?;
    let ai_service = AiService;
    let ai_user_id = ai_service
        .create_ai_template_user(
            shared.transaction(),
            "Test Bot Display",
            RandomPlayer::NAME,
            RandomPlayer::VERSION,
            None,
            Some(100),
        )
        .await?;

    // Update profile.display_name to match what add_ai_seat would set
    // (simulating the production flow where profile.display_name is set from friendly_ai_name)
    if let Some(mut profile) =
        ai_profiles::find_by_user_id(shared.transaction(), ai_user_id).await?
    {
        let expected_name = backend::routes::games::friendly_ai_name(ai_user_id, 2);
        profile.display_name = expected_name;
        ai_profiles::update_profile(shared.transaction(), profile).await?;
    }

    create_test_game_player(shared.transaction(), game_id, ai_user_id, 2).await?;

    // Create test app
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(web::scope("/api/games").configure(configure_routes));
        })
        .build()
        .await?;

    // Test the endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/players/2/display_name"))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let expected_name = backend::routes::games::friendly_ai_name(ai_user_id, 2);
    assert_eq!(body["display_name"], expected_name);

    shared.rollback().await?;

    Ok(())
}

#[tokio::test]
async fn test_get_player_display_name_not_found() -> Result<(), AppError> {
    let state = build_test_state().await?;

    // Get pooled DB and open a shared txn
    let db = backend::db::require_db(&state).expect("DB required for this test");
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    // Create test game but no players
    let game_id = create_test_game(shared.transaction()).await?;

    // Create test app
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(web::scope("/api/games").configure(configure_routes));
        })
        .build()
        .await?;

    // Test the endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/players/0/display_name"))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Validate error structure using centralized helper (includes trace_id validation)
    crate::common::assert_problem_details_structure(
        resp,
        404,
        "PLAYER_NOT_FOUND",
        "Player not found at seat",
    )
    .await;

    // Rollback the transaction after all assertions complete
    shared.rollback().await?;

    Ok(())
}

#[tokio::test]
async fn test_get_player_display_name_invalid_seat() -> Result<(), AppError> {
    let state = build_test_state().await?;

    // Get pooled DB and open a shared txn
    let db = backend::db::require_db(&state).expect("DB required for this test");
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    // Create test data
    let game_id = create_test_game(shared.transaction()).await?;
    let user_id = create_test_user(shared.transaction(), "alice", Some("AliceUser")).await?;
    create_test_game_player(shared.transaction(), game_id, user_id, 0).await?;

    // Create test app
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(web::scope("/api/games").configure(configure_routes));
        })
        .build()
        .await?;

    // Test the endpoint with invalid seat
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/players/5/display_name"))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Validate error structure using centralized helper (includes trace_id validation)
    crate::common::assert_problem_details_structure(
        resp,
        422,
        "INVALID_SEAT",
        "Seat must be between 0 and 3",
    )
    .await;

    // Rollback the transaction after all assertions complete
    shared.rollback().await?;

    Ok(())
}

#[tokio::test]
async fn test_get_player_display_name_fallback_to_sub() -> Result<(), AppError> {
    let state = build_test_state().await?;

    // Get pooled DB and open a shared txn
    let db = backend::db::require_db(&state).expect("DB required for this test");
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    // Create test data with no username (should fall back to sub)
    let game_id = create_test_game(shared.transaction()).await?;
    let user_id = create_test_user(shared.transaction(), "bob", None).await?;
    create_test_game_player(shared.transaction(), game_id, user_id, 1).await?;

    // Create test app
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(web::scope("/api/games").configure(configure_routes));
        })
        .build()
        .await?;

    // Test the endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/players/1/display_name"))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    // Parse response body
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["display_name"], "bob");

    // Rollback the transaction after all assertions complete
    shared.rollback().await?;

    Ok(())
}
