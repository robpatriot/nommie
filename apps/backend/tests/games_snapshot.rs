//! Integration tests for the GET /api/games/{game_id}/snapshot endpoint.
//!
//! Tests include:
//! - Basic 200 responses with valid JSON
//! - ETag header presence and format
//! - If-None-Match → 304 Not Modified caching
//! - Error cases (400, 404)

mod common;

use actix_web::http::header::{HeaderValue, IF_NONE_MATCH};
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

#[tokio::test]
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

#[tokio::test]
async fn test_snapshot_returns_etag_header() -> Result<(), AppError> {
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
        lock_version: Set(5), // Set specific lock_version to verify in ETag
        ..Default::default()
    };
    let game = game.insert(shared.transaction()).await?;
    let game_id = game.id;
    let lock_version = game.lock_version;

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
    let status = resp.status();
    assert_eq!(status, StatusCode::OK);

    // Extract and verify ETag header before consuming response
    let etag_str = resp
        .headers()
        .get("etag")
        .expect("ETag header should be present")
        .to_str()
        .expect("ETag should be valid ASCII string")
        .to_string(); // Clone the string before response is dropped

    // Consume response body to fully release the response
    let _body = test::read_body(resp).await;

    // Verify ETag format: "game-{id}-v{version}"
    let expected_etag = format!(r#""game-{game_id}-v{lock_version}""#);
    assert_eq!(
        etag_str, expected_etag,
        "ETag should be in format \"game-{{id}}-v{{version}}\""
    );

    // Verify we can parse the version back
    let parsed_version = backend::http::etag::parse_game_version_from_etag(&etag_str)
        .expect("Should be able to parse version from ETag");
    assert_eq!(
        parsed_version, lock_version,
        "Parsed version should match game lock_version"
    );

    // Cleanup — explicitly rollback the shared transaction
    shared.rollback().await?;

    Ok(())
}

#[tokio::test]
async fn test_snapshot_with_if_none_match_returns_304() -> Result<(), AppError> {
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
        lock_version: Set(3), // Set specific lock_version
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

    // First GET to capture ETag
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/snapshot"))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Extract ETag header
    let etag = resp
        .headers()
        .get("etag")
        .expect("ETag header should be present")
        .to_str()
        .expect("ETag should be valid ASCII string")
        .to_string();

    // Consume first response body
    let _body = test::read_body(resp).await;

    // Second GET with If-None-Match matching the ETag
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/snapshot"))
        .insert_header((IF_NONE_MATCH, HeaderValue::from_str(&etag).unwrap()))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Should return 304 Not Modified
    assert_eq!(
        resp.status(),
        StatusCode::NOT_MODIFIED,
        "Should return 304 when If-None-Match matches current ETag"
    );

    // ETag should still be present in 304 response
    let etag_304 = resp
        .headers()
        .get("etag")
        .expect("ETag header should be present in 304 response")
        .to_str()
        .expect("ETag should be valid ASCII string");

    assert_eq!(
        etag_304, etag,
        "ETag in 304 response should match original ETag"
    );

    // Body should be empty for 304
    let body = test::read_body(resp).await;
    assert!(
        body.is_empty(),
        "304 response should have empty body, got {} bytes",
        body.len()
    );

    // Cleanup — explicitly rollback the shared transaction
    shared.rollback().await?;

    Ok(())
}

#[tokio::test]
async fn test_snapshot_with_if_none_match_mismatch_returns_200() -> Result<(), AppError> {
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
        lock_version: Set(5), // Set specific lock_version
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

    // GET with If-None-Match that doesn't match (stale version)
    let stale_etag = format!(r#""game-{game_id}-v3""#); // Resource is at v5
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/snapshot"))
        .insert_header((IF_NONE_MATCH, HeaderValue::from_str(&stale_etag).unwrap()))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Should return 200 with full body since ETag doesn't match
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Should return 200 when If-None-Match doesn't match current ETag"
    );

    // Should have current ETag in response
    let current_etag = resp
        .headers()
        .get("etag")
        .expect("ETag header should be present")
        .to_str()
        .expect("ETag should be valid ASCII string");

    assert_eq!(
        current_etag,
        format!(r#""game-{game_id}-v5""#),
        "Should return current ETag"
    );

    // Body should be present
    let body = test::read_body(resp).await;
    assert!(
        !body.is_empty(),
        "200 response should have body with snapshot data"
    );

    // Verify body is valid JSON with expected structure
    let json: Value = serde_json::from_slice(&body).expect("Body should be valid JSON");
    assert!(json.get("game").is_some(), "Should have game field");
    assert!(json.get("phase").is_some(), "Should have phase field");

    // Cleanup — explicitly rollback the shared transaction
    shared.rollback().await?;

    Ok(())
}

#[tokio::test]
async fn test_snapshot_with_if_none_match_wildcard_returns_304() -> Result<(), AppError> {
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
        lock_version: Set(7), // Set specific lock_version
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

    // GET with If-None-Match: * (wildcard per RFC 9110)
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/snapshot"))
        .insert_header((IF_NONE_MATCH, HeaderValue::from_static("*")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Should return 304 Not Modified since wildcard matches any representation
    assert_eq!(
        resp.status(),
        StatusCode::NOT_MODIFIED,
        "Should return 304 when If-None-Match is wildcard"
    );

    // ETag should still be present in 304 response
    let etag_304 = resp
        .headers()
        .get("etag")
        .expect("ETag header should be present in 304 response")
        .to_str()
        .expect("ETag should be valid ASCII string");

    let expected_etag = format!(r#""game-{game_id}-v7""#);
    assert_eq!(
        etag_304, expected_etag,
        "ETag in 304 response should be the current version"
    );

    // Body should be empty for 304
    let body = test::read_body(resp).await;
    assert!(
        body.is_empty(),
        "304 response should have empty body, got {} bytes",
        body.len()
    );

    // Cleanup — explicitly rollback the shared transaction
    shared.rollback().await?;

    Ok(())
}

#[tokio::test]
async fn test_snapshot_with_if_none_match_comma_separated_one_match() -> Result<(), AppError> {
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
        lock_version: Set(10), // Current version is 10
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

    // GET with If-None-Match containing multiple ETags (one matches)
    let if_none_match_value =
        format!(r#""game-{game_id}-v8", "game-{game_id}-v9", "game-{game_id}-v10""#);
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/snapshot"))
        .insert_header((
            IF_NONE_MATCH,
            HeaderValue::from_str(&if_none_match_value).unwrap(),
        ))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Should return 304 since one of the ETags matches
    assert_eq!(
        resp.status(),
        StatusCode::NOT_MODIFIED,
        "Should return 304 when one ETag in comma-separated list matches"
    );

    // Consume response body before rollback
    let _body = test::read_body(resp).await;

    // Cleanup — explicitly rollback the shared transaction
    shared.rollback().await?;

    Ok(())
}

#[tokio::test]
async fn test_snapshot_with_if_none_match_comma_separated_no_match() -> Result<(), AppError> {
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
        lock_version: Set(15), // Current version is 15
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

    // GET with If-None-Match containing multiple ETags (none match)
    let if_none_match_value =
        format!(r#""game-{game_id}-v12", "game-{game_id}-v13", "game-{game_id}-v14""#);
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{game_id}/snapshot"))
        .insert_header((
            IF_NONE_MATCH,
            HeaderValue::from_str(&if_none_match_value).unwrap(),
        ))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Should return 200 with full body since none of the ETags match
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Should return 200 when no ETags in comma-separated list match"
    );

    // Should have current ETag in response
    let current_etag = resp
        .headers()
        .get("etag")
        .expect("ETag header should be present")
        .to_str()
        .expect("ETag should be valid ASCII string");

    assert_eq!(
        current_etag,
        format!(r#""game-{game_id}-v15""#),
        "Should return current ETag"
    );

    // Body should be present with snapshot data
    let body = test::read_body(resp).await;
    assert!(
        !body.is_empty(),
        "200 response should have body with snapshot data"
    );

    // Verify body is valid JSON with expected structure
    let json: Value = serde_json::from_slice(&body).expect("Body should be valid JSON");
    assert!(json.get("game").is_some(), "Should have game field");
    assert!(json.get("phase").is_some(), "Should have phase field");

    // Cleanup — explicitly rollback the shared transaction
    shared.rollback().await?;

    Ok(())
}
