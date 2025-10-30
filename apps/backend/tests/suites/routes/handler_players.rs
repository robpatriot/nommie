use actix_web::http::StatusCode;
use actix_web::{test, web, App, HttpMessage};
use backend::config::db::{DbKind, RuntimeEnv};
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::routes::games::configure_routes;

use crate::support::db_memberships::create_test_game_player;
use crate::support::factory::{create_test_game, create_test_user};

#[tokio::test]
async fn test_get_player_display_name_success() -> Result<(), AppError> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await
        .expect("build test state with DB");

    // Get pooled DB and open a shared txn
    let db = backend::db::require_db(&state).expect("DB required for this test");
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    // Create test data using the shared transaction
    let game_id = create_test_game(shared.transaction()).await?;
    let user_id = create_test_user(shared.transaction(), "alice", Some("AliceUser")).await?;
    create_test_game_player(shared.transaction(), game_id, user_id, 0).await?;

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(configure_routes),
    )
    .await;

    // Test the endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/players/0/display_name"))
        .to_request();

    // Inject the shared transaction into the request
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
async fn test_get_player_display_name_not_found() -> Result<(), AppError> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await
        .expect("build test state with DB");

    // Get pooled DB and open a shared txn
    let db = backend::db::require_db(&state).expect("DB required for this test");
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    // Create test game but no players
    let game_id = create_test_game(shared.transaction()).await?;

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(configure_routes),
    )
    .await;

    // Test the endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/players/0/display_name"))
        .to_request();

    // Inject the shared transaction into the request
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
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await
        .expect("build test state with DB");

    // Get pooled DB and open a shared txn
    let db = backend::db::require_db(&state).expect("DB required for this test");
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    // Create test data
    let game_id = create_test_game(shared.transaction()).await?;
    let user_id = create_test_user(shared.transaction(), "alice", Some("AliceUser")).await?;
    create_test_game_player(shared.transaction(), game_id, user_id, 0).await?;

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(configure_routes),
    )
    .await;

    // Test the endpoint with invalid seat
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/players/5/display_name"))
        .to_request();

    // Inject the shared transaction into the request
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
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await
        .expect("build test state with DB");

    // Get pooled DB and open a shared txn
    let db = backend::db::require_db(&state).expect("DB required for this test");
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    // Create test data with no username (should fall back to sub)
    let game_id = create_test_game(shared.transaction()).await?;
    let user_id = create_test_user(shared.transaction(), "bob", None).await?;
    create_test_game_player(shared.transaction(), game_id, user_id, 1).await?;

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(configure_routes),
    )
    .await;

    // Test the endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/players/1/display_name"))
        .to_request();

    // Inject the shared transaction into the request
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
