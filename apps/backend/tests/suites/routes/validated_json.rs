use std::sync::Arc;

use actix_web::test;
use backend::auth::google::{MockGoogleVerifier, VerifiedGoogleClaims};
use backend::auth::jwt::verify_access_token;
use backend::db::require_db;
use backend::state::security_config::SecurityConfig;
use backend_test_support::unique_helpers::{unique_email, unique_str};
use serde_json::json;

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;
use crate::support::auth::seed_admission_email;
use crate::support::test_state_builder;

#[actix_web::test]
async fn test_malformed_json_returns_400_with_rfc7807() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config)
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state).with_prod_routes().build().await?;

    // Test malformed JSON (trailing comma)
    let malformed_json = r#"{"id_token": "test",}"#;

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .insert_header(("content-type", "application/json"))
        .set_payload(malformed_json)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Validate error structure using centralized helper
    assert_problem_details_structure(resp, 400, "BAD_REQUEST", "Invalid JSON at line 1").await;

    Ok(())
}

#[actix_web::test]
async fn test_wrong_type_returns_400_with_rfc7807() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config)
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state).with_prod_routes().build().await?;

    // Test wrong type (number instead of string for id_token)
    let wrong_type_json = json!({
        "id_token": 123
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .insert_header(("content-type", "application/json"))
        .set_json(wrong_type_json)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Validate error structure using centralized helper
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
async fn test_missing_required_field_returns_400_with_rfc7807(
) -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config)
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state).with_prod_routes().build().await?;

    // Test missing required field (id_token is required but missing)
    let missing_field_json = json!({});

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .insert_header(("content-type", "application/json"))
        .set_json(missing_field_json)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Serde reports missing field as Data error -> "wrong types for one or more fields"
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
async fn test_valid_json_happy_path_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("test");
    let test_google_sub = unique_str("google");
    let mock_verifier = Arc::new(MockGoogleVerifier::new(VerifiedGoogleClaims {
        sub: test_google_sub,
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
    seed_admission_email(&db, &test_email.to_lowercase()).await;

    let app = create_test_app(state).with_prod_routes().build().await?;

    let valid_json = json!({ "id_token": "test-token" });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .insert_header(("content-type", "application/json"))
        .set_json(valid_json)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 200 OK as before
    assert!(resp.status().is_success());
    assert_eq!(resp.status().as_u16(), 200);

    // Should return JSON response with token
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("token").is_some());

    let token = body["token"].as_str().unwrap();
    assert!(!token.is_empty());

    // Verify the JWT can be decoded
    let decoded = verify_access_token(token, &security_config).expect("JWT should be valid");
    assert_eq!(decoded.email, test_email);

    Ok(())
}

#[actix_web::test]
async fn test_non_json_content_type_still_attempts_parse() -> Result<(), Box<dyn std::error::Error>>
{
    let test_email = unique_email("test");
    let test_google_sub = unique_str("google");
    let mock_verifier = Arc::new(MockGoogleVerifier::new(VerifiedGoogleClaims {
        sub: test_google_sub,
        email: test_email.clone(),
        name: Some("Test User".to_string()),
    }));

    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config)
        .with_google_verifier(mock_verifier)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required for this test");
    seed_admission_email(&db, &test_email.to_lowercase()).await;

    let app = create_test_app(state).with_prod_routes().build().await?;

    let valid_json = json!({ "id_token": "test-token" });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .insert_header(("content-type", "text/plain"))
        .set_json(valid_json)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should still work since we attempt to parse regardless of content type
    assert!(resp.status().is_success());
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("token").is_some());

    Ok(())
}

#[actix_web::test]
async fn test_empty_body_returns_400_with_rfc7807() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config)
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state).with_prod_routes().build().await?;

    // Test empty body
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .insert_header(("content-type", "application/json"))
        .set_payload("")
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Validate error structure using centralized helper
    assert_problem_details_structure(
        resp,
        400,
        "BAD_REQUEST",
        "Invalid JSON: unexpected end of input",
    )
    .await;

    Ok(())
}
