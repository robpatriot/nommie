mod common;
mod support;

use actix_web::test;
use backend::config::db::DbProfile;
use backend::infra::state::build_state;
use backend::state::security_config::SecurityConfig;
use backend::utils::unique::{unique_email, unique_str};
use serde_json::json;
use support::create_test_app;

#[actix_web::test]
async fn test_malformed_json_returns_400_with_rfc7807() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config)
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state).with_prod_routes().build().await?;

    // Test malformed JSON (trailing comma)
    let malformed_json = r#"{"email": "test@example.com", "google_sub": "test_sub",}"#;

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .insert_header(("content-type", "application/json"))
        .set_payload(malformed_json)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 400 Bad Request
    assert_eq!(resp.status().as_u16(), 400);

    // Assert headers
    let content_type = resp.headers().get("content-type").unwrap();
    assert_eq!(content_type.to_str().unwrap(), "application/problem+json");

    // Negative header checks - these should NOT be present for 400 errors
    assert!(resp.headers().get("www-authenticate").is_none());
    assert!(resp.headers().get("retry-after").is_none());

    // Assert RFC7807 body structure
    let body: serde_json::Value = test::read_body_json(resp).await;

    assert!(body.get("type").is_some());
    assert!(body.get("title").is_some());
    assert_eq!(body["status"], 400);
    assert!(body.get("detail").is_some());
    assert!(body.get("code").is_some());
    assert!(body.get("trace_id").is_some());

    // Verify the error code is the expected validation error code
    assert_eq!(body["code"], "BAD_REQUEST");

    // Verify detail contains stable substrings about JSON parsing
    let detail = body["detail"].as_str().unwrap();
    assert!(detail.contains("Invalid JSON"));

    // Verify trace_id is present and non-empty
    let trace_id = body["trace_id"].as_str().unwrap();
    assert!(!trace_id.is_empty());

    Ok(())
}

#[actix_web::test]
async fn test_wrong_type_returns_400_with_rfc7807() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config)
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state).with_prod_routes().build().await?;

    // Test wrong type (number instead of string for email)
    let wrong_type_json = json!({
        "email": 123,
        "google_sub": "test_sub"
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .insert_header(("content-type", "application/json"))
        .set_json(wrong_type_json)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 400 Bad Request
    assert_eq!(resp.status().as_u16(), 400);

    // Assert headers
    let content_type = resp.headers().get("content-type").unwrap();
    assert_eq!(content_type.to_str().unwrap(), "application/problem+json");

    // Negative header checks
    assert!(resp.headers().get("www-authenticate").is_none());
    assert!(resp.headers().get("retry-after").is_none());

    // Assert RFC7807 body structure
    let body: serde_json::Value = test::read_body_json(resp).await;

    assert_eq!(body["status"], 400);
    assert_eq!(body["code"], "BAD_REQUEST");

    // Verify detail contains information about wrong types
    let detail = body["detail"].as_str().unwrap();
    assert!(detail.contains("Invalid JSON"));
    assert!(detail.contains("wrong types") || detail.contains("field"));

    // Verify trace_id is present and non-empty
    let trace_id = body["trace_id"].as_str().unwrap();
    assert!(!trace_id.is_empty());

    Ok(())
}

#[actix_web::test]
async fn test_missing_required_field_returns_400_with_rfc7807(
) -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config)
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state).with_prod_routes().build().await?;

    // Test missing required field (google_sub is required but missing)
    let missing_field_json = json!({
        "email": "test@example.com"
        // Missing google_sub field
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .insert_header(("content-type", "application/json"))
        .set_json(missing_field_json)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 400 Bad Request
    assert_eq!(resp.status().as_u16(), 400);

    // Assert headers
    let content_type = resp.headers().get("content-type").unwrap();
    assert_eq!(content_type.to_str().unwrap(), "application/problem+json");

    // Negative header checks
    assert!(resp.headers().get("www-authenticate").is_none());
    assert!(resp.headers().get("retry-after").is_none());

    // Assert RFC7807 body structure
    let body: serde_json::Value = test::read_body_json(resp).await;

    assert_eq!(body["status"], 400);
    // This should be INVALID_GOOGLE_SUB because the JSON is valid but the field is missing
    assert_eq!(body["code"], "INVALID_GOOGLE_SUB");

    // Verify detail contains information about missing field
    let detail = body["detail"].as_str().unwrap();
    assert!(detail.contains("Google sub cannot be empty"));

    // Verify trace_id is present and non-empty
    let trace_id = body["trace_id"].as_str().unwrap();
    assert!(!trace_id.is_empty());

    Ok(())
}

#[actix_web::test]
async fn test_valid_json_happy_path_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config.clone())
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state).with_prod_routes().build().await?;

    // Test valid JSON - should work as before
    let test_email = unique_email("test");
    let test_google_sub = unique_str("google");
    let valid_json = json!({
        "email": test_email,
        "name": "Test User",
        "google_sub": test_google_sub
    });

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
    let decoded =
        backend::verify_access_token(token, &security_config).expect("JWT should be valid");
    assert_eq!(decoded.email, test_email);

    Ok(())
}

#[actix_web::test]
async fn test_non_json_content_type_still_attempts_parse() -> Result<(), Box<dyn std::error::Error>>
{
    // Build state with database and security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config)
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state).with_prod_routes().build().await?;

    // Test with non-JSON content type but valid JSON body
    let test_email = unique_email("test");
    let test_google_sub = unique_str("google");
    let valid_json = json!({
        "email": test_email,
        "name": "Test User",
        "google_sub": test_google_sub
    });

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
    let state = build_state()
        .with_db(DbProfile::Test)
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

    // Should return 400 Bad Request
    assert_eq!(resp.status().as_u16(), 400);

    // Assert headers
    let content_type = resp.headers().get("content-type").unwrap();
    assert_eq!(content_type.to_str().unwrap(), "application/problem+json");

    // Negative header checks
    assert!(resp.headers().get("www-authenticate").is_none());
    assert!(resp.headers().get("retry-after").is_none());

    // Assert RFC7807 body structure
    let body: serde_json::Value = test::read_body_json(resp).await;

    assert_eq!(body["status"], 400);
    assert_eq!(body["code"], "BAD_REQUEST");

    // Verify detail contains information about empty input
    let detail = body["detail"].as_str().unwrap();
    assert!(detail.contains("Invalid JSON"));
    assert!(detail.contains("end of input") || detail.contains("EOF"));

    // Verify trace_id is present and non-empty
    let trace_id = body["trace_id"].as_str().unwrap();
    assert!(!trace_id.is_empty());

    Ok(())
}
