// Integration tests for auth login endpoint.

use std::sync::Arc;

use actix_web::{test, HttpMessage};
use backend::auth::google::{MockGoogleVerifier, VerifiedGoogleClaims};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::infra::state::build_state;
use backend_test_support::unique_helpers::{unique_email, unique_str};
use serde_json::json;

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;
use crate::support::auth::seed_admission_email;
use crate::support::test_state_builder;

// ============================================================================
// Happy Path Tests
// ============================================================================

#[actix_web::test]
async fn test_login_creates_and_reuses_user() -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("test");
    let test_google_sub = unique_str("google");
    let mock_verifier = Arc::new(MockGoogleVerifier::new(VerifiedGoogleClaims {
        sub: test_google_sub.clone(),
        email: test_email.clone(),
        name: Some("Test User".to_string()),
    }));

    let state = test_state_builder()?
        .with_google_verifier(mock_verifier)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required for this test");
    seed_admission_email(&db, &test_email.to_lowercase(), false).await;
    let shared = SharedTxn::open(&db).await?;

    let app = create_test_app(state).with_prod_routes().build().await?;

    let login_data = json!({ "id_token": "test-token" });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("token").is_some());

    let token = body["token"].as_str().unwrap();
    assert!(!token.is_empty());
    // Session token is a 32-char hex string (UUID v4 simple format)
    assert_eq!(token.len(), 32);

    let first_token = token.to_string();

    let login_data_2 = json!({ "id_token": "test-token" });
    let req2 = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data_2)
        .to_request();
    req2.extensions_mut().insert(shared.clone());

    let resp2 = test::call_service(&app, req2).await;

    assert!(resp2.status().is_success());
    assert_eq!(resp2.status().as_u16(), 200);

    let body2: serde_json::Value = test::read_body_json(resp2).await;
    let token2 = body2["token"].as_str().unwrap();
    // Each login creates a new session token (they should differ)
    assert_ne!(token2, first_token);
    assert_eq!(token2.len(), 32);

    shared.rollback().await?;

    Ok(())
}

// ============================================================================
// Validation Error Tests
// ============================================================================

#[actix_web::test]
async fn test_login_rejects_missing_id_token() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state().build().await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let login_data = json!({});

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_problem_details_structure(
        resp,
        400,
        "BAD_REQUEST",
        "Invalid JSON: wrong types for one or more fields",
    )
    .await;

    Ok(())
}

#[actix_web::test]
async fn test_login_rejects_empty_id_token() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state().build().await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let login_data = json!({ "id_token": "" });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_problem_details_structure(resp, 400, "INVALID_ID_TOKEN", "id_token cannot be empty")
        .await;

    Ok(())
}

#[actix_web::test]
async fn test_login_rejects_wrong_type_for_id_token() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state().build().await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let login_data = json!({ "id_token": 123 });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_problem_details_structure(
        resp,
        400,
        "BAD_REQUEST",
        "Invalid JSON: wrong types for one or more fields",
    )
    .await;

    Ok(())
}

// ============================================================================
// Logout Tests
// ============================================================================

#[actix_web::test]
async fn test_logout_returns_200() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state().build().await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let req = test::TestRequest::post()
        .uri("/api/auth/logout")
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Logout always returns 200 regardless of whether cookie was present
    assert_eq!(resp.status().as_u16(), 200);

    Ok(())
}
