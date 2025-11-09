// Integration tests for GET /api/games/{game_id}/snapshot endpoint.
//
// Tests include:
// - Basic 200 responses with valid JSON
// - Phase structure validation
// - Error cases (400, 404)

use actix_web::http::StatusCode;
use actix_web::{test, web, HttpMessage};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::routes::games;
use serde_json::Value;

use crate::support::app_builder::create_test_app;
use crate::support::build_test_state;
use crate::support::snapshot_helpers::{create_snapshot_game, SnapshotGameOptions};

#[tokio::test]
async fn test_snapshot_returns_200_with_valid_json() -> Result<(), AppError> {
    let state = build_test_state().await?;

    let db = require_db(&state)?;
    let shared = SharedTxn::open(db).await?;

    let game = create_snapshot_game(&shared, SnapshotGameOptions::default()).await?;

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(web::scope("/api/games").configure(games::configure_routes));
        })
        .build()
        .await?;

    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{}/snapshot", game.game_id))
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
    let game_obj = json.get("game").unwrap();
    assert!(
        game_obj.get("round_no").is_some(),
        "game should have round_no"
    );
    assert!(game_obj.get("dealer").is_some(), "game should have dealer");
    assert!(
        game_obj.get("seating").is_some(),
        "game should have seating"
    );
    assert!(
        game_obj.get("scores_total").is_some(),
        "game should have scores_total"
    );

    // Verify phase structure (should have a phase tag and data)
    let phase = json.get("phase").unwrap();
    assert!(
        phase.get("phase").is_some(),
        "phase should have discriminator tag"
    );

    shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_snapshot_invalid_game_id_returns_400() -> Result<(), AppError> {
    let state = build_state().build().await?;

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(web::scope("/api/games").configure(games::configure_routes));
        })
        .build()
        .await?;

    // Call with invalid game_id (not a number)
    let req = test::TestRequest::get()
        .uri("/api/games/not-a-number/snapshot")
        .to_request();

    let resp = test::call_service(&app, req).await;

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
async fn test_snapshot_nonexistent_game_returns_404() -> Result<(), AppError> {
    let state = build_test_state().await?;

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(web::scope("/api/games").configure(games::configure_routes));
        })
        .build()
        .await?;

    // Call with a valid ID format but nonexistent game (very large ID)
    let req = test::TestRequest::get()
        .uri("/api/games/999999999/snapshot")
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 404 status
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let body = test::read_body(resp).await;
    if !body.is_empty() {
        let json: Value = serde_json::from_slice(&body).expect("Valid JSON response");
        assert_eq!(
            json.get("code").and_then(|v| v.as_str()),
            Some("GAME_NOT_FOUND")
        );
        assert!(json.get("trace_id").is_some(), "error should have trace_id");
        assert_eq!(json.get("status").and_then(|v| v.as_u64()), Some(404));
    }

    Ok(())
}

#[tokio::test]
async fn test_snapshot_phase_structure() -> Result<(), AppError> {
    let state = build_test_state().await?;

    let db = require_db(&state)?;
    let shared = SharedTxn::open(db).await?;

    let game = create_snapshot_game(&shared, SnapshotGameOptions::default()).await?;

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(web::scope("/api/games").configure(games::configure_routes));
        })
        .build()
        .await?;

    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{}/snapshot", game.game_id))
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

    shared.rollback().await?;
    Ok(())
}
