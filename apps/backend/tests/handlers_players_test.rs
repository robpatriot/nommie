mod common;
mod support;

use actix_web::http::StatusCode;
use actix_web::{test, web, App, HttpMessage};
use backend::entities::{game_players, games, users};
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::routes::games::configure_routes;
use sea_orm::{ActiveModelTrait, Set};

#[tokio::test]
async fn test_get_player_display_name_success() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
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

    Ok(())
}

#[tokio::test]
async fn test_get_player_display_name_not_found() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
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
    common::assert_problem_details_structure(
        resp,
        404,
        "PLAYER_NOT_FOUND",
        "Player not found at seat",
    )
    .await;

    Ok(())
}

#[tokio::test]
async fn test_get_player_display_name_invalid_seat() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
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
    common::assert_problem_details_structure(
        resp,
        422,
        "INVALID_SEAT",
        "Seat must be between 0 and 3",
    )
    .await;

    Ok(())
}

#[tokio::test]
async fn test_get_player_display_name_fallback_to_sub() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
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

    Ok(())
}

// Helper functions for test data creation

async fn create_test_game(txn: &impl sea_orm::ConnectionTrait) -> Result<i64, AppError> {
    // Create a user first to use as created_by
    let user_id = create_test_user(txn, "creator", Some("Creator")).await?;

    let game = games::ActiveModel {
        id: sea_orm::NotSet,
        created_by: Set(Some(user_id)),
        visibility: Set(games::GameVisibility::Public),
        state: Set(games::GameState::Lobby),
        created_at: Set(time::OffsetDateTime::now_utc()),
        updated_at: Set(time::OffsetDateTime::now_utc()),
        started_at: Set(None),
        ended_at: Set(None),
        name: Set(Some("Test Game".to_string())),
        join_code: Set(Some("ABC123".to_string())),
        rules_version: Set("1.0".to_string()),
        rng_seed: Set(Some(12345)),
        current_round: Set(Some(1)),
        hand_size: Set(Some(13)),
        dealer_pos: Set(Some(0)),
        lock_version: Set(1),
    };

    let inserted = game.insert(txn).await?;
    Ok(inserted.id)
}

async fn create_test_user(
    txn: &impl sea_orm::ConnectionTrait,
    sub: &str,
    username: Option<&str>,
) -> Result<i64, AppError> {
    let user = users::ActiveModel {
        id: sea_orm::NotSet,
        sub: Set(sub.to_string()),
        username: Set(username.map(|s| s.to_string())),
        is_ai: Set(false),
        created_at: Set(time::OffsetDateTime::now_utc()),
        updated_at: Set(time::OffsetDateTime::now_utc()),
    };

    let inserted = user.insert(txn).await?;
    Ok(inserted.id)
}

async fn create_test_game_player(
    txn: &impl sea_orm::ConnectionTrait,
    game_id: i64,
    user_id: i64,
    turn_order: i32,
) -> Result<i64, AppError> {
    let now = time::OffsetDateTime::now_utc();
    let game_player = game_players::ActiveModel {
        id: sea_orm::NotSet,
        game_id: Set(game_id),
        user_id: Set(user_id),
        turn_order: Set(turn_order),
        is_ready: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let result = game_player.insert(txn).await?;
    println!("Test: Inserted game_player with id={}", result.id);
    Ok(result.id)
}
