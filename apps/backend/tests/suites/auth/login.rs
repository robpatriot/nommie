// Integration tests for auth login endpoint.
//
// Tests login with verified Google ID token and refresh endpoint.

use std::sync::Arc;

use actix_web::{test, HttpMessage};
use backend::auth::google::{MockGoogleVerifier, VerifiedGoogleClaims};
use backend::auth::jwt::verify_access_token;
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::infra::state::build_state;
use backend::state::security_config::SecurityConfig;
use backend_test_support::unique_helpers::{unique_email, unique_str};
use serde_json::json;

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;
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

    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config.clone())
        .with_google_verifier(mock_verifier)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required for this test");
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

    let decoded = verify_access_token(token, &security_config).expect("JWT should be valid");
    assert_eq!(decoded.email, test_email);

    let first_user_id_str = decoded.sub;

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
    let decoded2 = verify_access_token(token2, &security_config).expect("JWT should be valid");

    assert_eq!(
        decoded2.sub, first_user_id_str,
        "repeat login should return same user id"
    );
    assert_eq!(decoded2.email, test_email);

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
// Refresh Endpoint Tests
// ============================================================================

#[actix_web::test]
async fn test_refresh_returns_new_jwt() -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("refresh");
    let test_google_sub = unique_str("refresh-google");
    let mock_verifier = Arc::new(MockGoogleVerifier::new(VerifiedGoogleClaims {
        sub: test_google_sub,
        email: test_email.clone(),
        name: None,
    }));

    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config.clone())
        .with_google_verifier(mock_verifier)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;

    let app = create_test_app(state).with_prod_routes().build().await?;

    let login_data = json!({ "id_token": "test-token" });
    let login_req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();
    login_req.extensions_mut().insert(shared.clone());

    let login_resp = test::call_service(&app, login_req).await;
    assert!(login_resp.status().is_success());
    let body: serde_json::Value = test::read_body_json(login_resp).await;
    let original_token = body["token"].as_str().unwrap();

    let refresh_req = test::TestRequest::post()
        .uri("/api/auth/refresh")
        .insert_header(("Authorization", format!("Bearer {}", original_token)))
        .to_request();

    let refresh_resp = test::call_service(&app, refresh_req).await;
    assert!(refresh_resp.status().is_success());
    let refresh_body: serde_json::Value = test::read_body_json(refresh_resp).await;
    let new_token = refresh_body["token"].as_str().unwrap();
    assert!(!new_token.is_empty());

    let decoded = verify_access_token(new_token, &security_config).expect("JWT should be valid");
    assert_eq!(decoded.email, test_email);

    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_refresh_rejects_missing_bearer() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state().build().await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let req = test::TestRequest::post()
        .uri("/api/auth/refresh")
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_problem_details_structure(
        resp,
        401,
        "UNAUTHORIZED_MISSING_BEARER",
        "Missing or malformed Bearer token",
    )
    .await;

    Ok(())
}
