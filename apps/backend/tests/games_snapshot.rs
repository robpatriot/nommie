//! Integration tests for the GET /api/games/{game_id}/snapshot endpoint.

mod common;

use actix_web::http::StatusCode;
use actix_web::{test, HttpMessage};
use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::routes;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_snapshot_returns_200_with_valid_json() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    // Get pooled DB and open a shared txn
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Create a game in the database using the shared transaction
    let now = time::OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Bidding),
        rules_version: Set("nommie-1.0.0".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let game = game.insert(shared.transaction()).await?;
    let game_id = game.id;

    // Build test app
    let app_state = actix_web::web::Data::new(state);
    let app = test::init_service(
        actix_web::App::new()
            .app_data(app_state.clone())
            .configure(routes::configure),
    )
    .await;

    // Call the snapshot endpoint with injected shared transaction
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/snapshot"))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Assert 200 status
    assert_eq!(resp.status(), StatusCode::OK);

    // Parse response body and assert JSON structure
    let body = test::read_body(resp).await;
    let json: Value = serde_json::from_slice(&body).expect("Valid JSON response");

    // Verify top-level structure
    assert!(
        json.get("game").is_some(),
        "snapshot should have 'game' field"
    );
    assert!(
        json.get("phase").is_some(),
        "snapshot should have 'phase' field"
    );

    // Verify game header fields
    let game = json.get("game").unwrap();
    assert!(game.get("round_no").is_some(), "game should have round_no");
    assert!(game.get("dealer").is_some(), "game should have dealer");
    assert!(game.get("seating").is_some(), "game should have seating");
    assert!(
        game.get("scores_total").is_some(),
        "game should have scores_total"
    );

    // Verify phase structure (should have a phase tag and data)
    let phase = json.get("phase").unwrap();
    assert!(
        phase.get("phase").is_some(),
        "phase should have discriminator tag"
    );

    // Cleanup — explicitly rollback the shared transaction
    shared.rollback().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_snapshot_invalid_game_id_returns_400() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    let app_state = actix_web::web::Data::new(state);
    let app = test::init_service(
        actix_web::App::new()
            .app_data(app_state.clone())
            .configure(routes::configure),
    )
    .await;

    // Call with invalid game_id (not a number)
    let req = test::TestRequest::get()
        .uri("/api/games/not-a-number/snapshot")
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 400 status
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Parse response body and verify error structure
    let body = test::read_body(resp).await;
    let json: Value = serde_json::from_slice(&body).expect("Valid JSON response");

    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some("INVALID_GAME_ID")
    );
    assert!(json.get("trace_id").is_some(), "error should have trace_id");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_snapshot_nonexistent_game_returns_404() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    let app_state = actix_web::web::Data::new(state);
    let app = test::init_service(
        actix_web::App::new()
            .app_data(app_state.clone())
            .configure(routes::configure),
    )
    .await;

    // Call with a valid ID format but nonexistent game (very large ID)
    let req = test::TestRequest::get()
        .uri("/api/games/999999999/snapshot")
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 404 status
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Parse response body and verify error structure
    let body = test::read_body(resp).await;
    let json: Value = serde_json::from_slice(&body).expect("Valid JSON response");

    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some("GAME_NOT_FOUND")
    );
    assert!(json.get("trace_id").is_some(), "error should have trace_id");
    assert_eq!(json.get("status").and_then(|v| v.as_u64()), Some(404));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_snapshot_phase_structure() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    // Get pooled DB and open a shared txn
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Create a game in the database using the shared transaction
    let now = time::OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Bidding),
        rules_version: Set("nommie-1.0.0".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let game = game.insert(shared.transaction()).await?;
    let game_id = game.id;

    let app_state = actix_web::web::Data::new(state);
    let app = test::init_service(
        actix_web::App::new()
            .app_data(app_state.clone())
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/snapshot"))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = test::read_body(resp).await;
    let json: Value = serde_json::from_slice(&body).expect("Valid JSON response");

    // Verify phase is Bidding and contains expected fields
    let phase_obj = json.get("phase").unwrap();
    assert_eq!(
        phase_obj.get("phase").and_then(|v| v.as_str()),
        Some("Bidding")
    );

    // Verify Bidding data exists
    let phase_data = phase_obj.get("data").unwrap();
    assert!(
        phase_data.get("round").is_some(),
        "Bidding should have round"
    );
    assert!(
        phase_data.get("to_act").is_some(),
        "Bidding should have to_act"
    );
    assert!(phase_data.get("bids").is_some(), "Bidding should have bids");
    assert!(
        phase_data.get("min_bid").is_some(),
        "Bidding should have min_bid"
    );
    assert!(
        phase_data.get("max_bid").is_some(),
        "Bidding should have max_bid"
    );

    // Cleanup — explicitly rollback the shared transaction
    shared.rollback().await?;

    Ok(())
}
