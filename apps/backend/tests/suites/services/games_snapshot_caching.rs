// Integration tests for GET /api/games/{game_id}/snapshot caching behavior.
//
// Tests include:
// - ETag header presence and format
// - If-None-Match → 304 Not Modified caching
// - Wildcard and comma-separated ETag handling

use actix_web::http::header::{HeaderValue, IF_NONE_MATCH};
use actix_web::http::StatusCode;
use actix_web::{test, web, HttpMessage};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::error::AppError;
use backend::middleware::jwt_extract::JwtExtract;
use backend::routes::games;
use backend::state::app_state::AppState;
use serde_json::Value;

use crate::support::app_builder::create_test_app;
use crate::support::auth::bearer_header;
use crate::support::build_test_state;
use crate::support::factory::create_test_user;
use crate::support::snapshot_helpers::{create_snapshot_game, SnapshotGameOptions};
use crate::support::test_utils::test_user_sub;

struct SnapshotTestContext {
    state: AppState,
    shared: SharedTxn,
    bearer: String,
}

async fn setup_snapshot_test(test_name: &str) -> Result<SnapshotTestContext, AppError> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = SharedTxn::open(db).await?;

    let viewer_sub = test_user_sub(&format!("{test_name}_viewer"));
    let viewer_email = format!("{viewer_sub}@example.com");
    create_test_user(shared.transaction(), &viewer_sub, Some("Snapshot Viewer")).await?;

    let bearer = bearer_header(&viewer_sub, &viewer_email, &security);

    Ok(SnapshotTestContext {
        state,
        shared,
        bearer,
    })
}

#[tokio::test]
async fn test_snapshot_returns_etag_header() -> Result<(), AppError> {
    let SnapshotTestContext {
        state,
        shared,
        bearer,
    } = setup_snapshot_test("snapshot_returns_etag_header").await?;

    let game =
        create_snapshot_game(&shared, SnapshotGameOptions::default().with_lock_version(5)).await?;

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games::configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{}/snapshot", game.game_id))
        .insert_header(("Authorization", bearer.clone()))
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
        .to_string();

    // Consume response body to fully release the response
    let _body = test::read_body(resp).await;

    // Verify ETag format: "game-{id}-v{version}"
    let expected_etag = format!(r#""game-{}-v{}""#, game.game_id, game.lock_version);
    assert_eq!(
        etag_str, expected_etag,
        "ETag should be in format \"game-{{id}}-v{{version}}\""
    );

    // Verify we can parse the version back
    let parsed_version = backend::http::etag::parse_game_version_from_etag(&etag_str)
        .expect("Should be able to parse version from ETag");
    assert_eq!(
        parsed_version, game.lock_version,
        "Parsed version should match game lock_version"
    );

    shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_snapshot_with_if_none_match_returns_304() -> Result<(), AppError> {
    let SnapshotTestContext {
        state,
        shared,
        bearer,
    } = setup_snapshot_test("snapshot_with_if_none_match_returns_304").await?;

    let game =
        create_snapshot_game(&shared, SnapshotGameOptions::default().with_lock_version(3)).await?;

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games::configure_routes),
            );
        })
        .build()
        .await?;

    // First GET to capture ETag
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{}/snapshot", game.game_id))
        .insert_header(("Authorization", bearer.clone()))
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
        .uri(&format!("/api/games/{}/snapshot", game.game_id))
        .insert_header((IF_NONE_MATCH, HeaderValue::from_str(&etag).unwrap()))
        .insert_header(("Authorization", bearer.clone()))
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

    shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_snapshot_with_if_none_match_mismatch_returns_200() -> Result<(), AppError> {
    let SnapshotTestContext {
        state,
        shared,
        bearer,
    } = setup_snapshot_test("snapshot_with_if_none_match_mismatch_returns_200").await?;

    let game =
        create_snapshot_game(&shared, SnapshotGameOptions::default().with_lock_version(5)).await?;

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games::configure_routes),
            );
        })
        .build()
        .await?;

    // GET with If-None-Match that doesn't match (stale version)
    let stale_etag = format!(r#""game-{}-v3""#, game.game_id); // Resource is at v5
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{}/snapshot", game.game_id))
        .insert_header((IF_NONE_MATCH, HeaderValue::from_str(&stale_etag).unwrap()))
        .insert_header(("Authorization", bearer.clone()))
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
        format!(r#""game-{}-v5""#, game.game_id),
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

    shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_snapshot_with_if_none_match_wildcard_returns_304() -> Result<(), AppError> {
    let SnapshotTestContext {
        state,
        shared,
        bearer,
    } = setup_snapshot_test("snapshot_with_if_none_match_wildcard_returns_304").await?;

    let game =
        create_snapshot_game(&shared, SnapshotGameOptions::default().with_lock_version(7)).await?;

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games::configure_routes),
            );
        })
        .build()
        .await?;

    // GET with If-None-Match: * (wildcard per RFC 9110)
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{}/snapshot", game.game_id))
        .insert_header((IF_NONE_MATCH, HeaderValue::from_static("*")))
        .insert_header(("Authorization", bearer.clone()))
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

    let expected_etag = format!(r#""game-{}-v7""#, game.game_id);
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

    shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_snapshot_with_if_none_match_comma_separated_one_match() -> Result<(), AppError> {
    let SnapshotTestContext {
        state,
        shared,
        bearer,
    } = setup_snapshot_test("snapshot_with_if_none_match_comma_separated_one_match").await?;

    let game = create_snapshot_game(
        &shared,
        SnapshotGameOptions::default().with_lock_version(10),
    )
    .await?;

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games::configure_routes),
            );
        })
        .build()
        .await?;

    // GET with If-None-Match containing multiple ETags (one matches)
    let if_none_match_value = format!(
        r#""game-{}-v8", "game-{}-v9", "game-{}-v10""#,
        game.game_id, game.game_id, game.game_id
    );
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{}/snapshot", game.game_id))
        .insert_header((
            IF_NONE_MATCH,
            HeaderValue::from_str(&if_none_match_value).unwrap(),
        ))
        .insert_header(("Authorization", bearer.clone()))
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

    shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn test_snapshot_with_if_none_match_comma_separated_no_match() -> Result<(), AppError> {
    let SnapshotTestContext {
        state,
        shared,
        bearer,
    } = setup_snapshot_test("snapshot_with_if_none_match_comma_separated_no_match").await?;

    let game = create_snapshot_game(
        &shared,
        SnapshotGameOptions::default().with_lock_version(15),
    )
    .await?;

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(games::configure_routes),
            );
        })
        .build()
        .await?;

    // GET with If-None-Match containing multiple ETags (none match)
    let if_none_match_value = format!(
        r#""game-{}-v12", "game-{}-v13", "game-{}-v14""#,
        game.game_id, game.game_id, game.game_id
    );
    let req = test::TestRequest::get()
        .uri(&format!("/api/games/{}/snapshot", game.game_id))
        .insert_header((
            IF_NONE_MATCH,
            HeaderValue::from_str(&if_none_match_value).unwrap(),
        ))
        .insert_header(("Authorization", bearer.clone()))
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
        format!(r#""game-{}-v15""#, game.game_id),
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

    shared.rollback().await?;
    Ok(())
}
