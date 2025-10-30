// Integration tests for auth login endpoint.
//
// Tests both successful login flows and validation errors.

use actix_web::test;
use backend::config::db::{DbKind, RuntimeEnv};
use backend::infra::state::build_state;
use backend::state::security_config::SecurityConfig;
use backend::utils::unique::{unique_email, unique_str};
use serde_json::json;

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;

// ============================================================================
// Happy Path Tests
// ============================================================================

#[actix_web::test]
async fn test_login_creates_and_reuses_user() -> Result<(), Box<dyn std::error::Error>> {
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .with_security(security_config.clone())
        .build()
        .await?;

    let app = create_test_app(state).with_prod_routes().build().await?;

    // First login with new email -> creates user + returns JWT
    let test_email = unique_email("test");
    let test_google_sub = unique_str("google");
    let login_data = json!({
        "email": test_email,
        "name": "Test User",
        "google_sub": test_google_sub
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("token").is_some());

    let token = body["token"].as_str().unwrap();
    assert!(!token.is_empty());

    let decoded =
        backend::verify_access_token(token, &security_config).expect("JWT should be valid");
    assert_eq!(decoded.email, test_email);

    let first_user_sub = decoded.sub;

    // Second call with same email -> reuses the same user
    let login_data_2 = json!({
        "email": test_email,
        "name": "Updated Name",
        "google_sub": test_google_sub
    });

    let req2 = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data_2)
        .to_request();

    let resp2 = test::call_service(&app, req2).await;

    assert!(resp2.status().is_success());
    assert_eq!(resp2.status().as_u16(), 200);

    let body2: serde_json::Value = test::read_body_json(resp2).await;
    let token2 = body2["token"].as_str().unwrap();

    let decoded2 =
        backend::verify_access_token(token2, &security_config).expect("JWT should be valid");

    assert_eq!(decoded2.sub, first_user_sub);
    assert_eq!(decoded2.email, test_email);

    Ok(())
}

// ============================================================================
// Validation Error Tests
// ============================================================================

#[actix_web::test]
async fn test_login_rejects_missing_required_fields() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let test_email = unique_email("test");
    let login_data = json!({
        "email": test_email
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_problem_details_structure(
        resp,
        400,
        "INVALID_GOOGLE_SUB",
        "Google sub cannot be empty",
    )
    .await;

    Ok(())
}

#[actix_web::test]
async fn test_login_rejects_empty_email() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let test_google_sub = unique_str("google");
    let login_data = json!({
        "email": "",
        "google_sub": test_google_sub
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_problem_details_structure(resp, 400, "INVALID_EMAIL", "Email cannot be empty").await;

    Ok(())
}

#[actix_web::test]
async fn test_login_rejects_empty_google_sub() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let test_email = unique_email("test");
    let login_data = json!({
        "email": test_email,
        "google_sub": "",
        "name": "Test User"
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_problem_details_structure(
        resp,
        400,
        "INVALID_GOOGLE_SUB",
        "Google sub cannot be empty",
    )
    .await;

    Ok(())
}

#[actix_web::test]
async fn test_login_rejects_both_empty() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let login_data = json!({
        "email": "",
        "google_sub": "",
        "name": ""
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should fail on first validation (email)
    assert_problem_details_structure(resp, 400, "INVALID_EMAIL", "Email cannot be empty").await;

    Ok(())
}

#[actix_web::test]
async fn test_login_missing_email_field() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let test_google_sub = unique_str("google");
    let login_data = json!({
        "google_sub": test_google_sub,
        "name": "Test User"
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_problem_details_structure(resp, 400, "INVALID_EMAIL", "Email cannot be empty").await;

    Ok(())
}

#[actix_web::test]
async fn test_login_missing_google_sub_field() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let test_email = unique_email("test");
    let login_data = json!({
        "email": test_email,
        "name": "Test User"
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_problem_details_structure(
        resp,
        400,
        "INVALID_GOOGLE_SUB",
        "Google sub cannot be empty",
    )
    .await;

    Ok(())
}

#[actix_web::test]
async fn test_login_wrong_type_for_email() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let test_google_sub = unique_str("google");
    let login_data = json!({
        "email": 123,
        "google_sub": test_google_sub,
        "name": "Test User"
    });

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
async fn test_login_wrong_type_for_google_sub() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let test_email = unique_email("test");
    let login_data = json!({
        "email": test_email,
        "google_sub": 456,
        "name": "Test User"
    });

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
async fn test_login_wrong_type_for_name() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let test_email = unique_email("test");
    let test_google_sub = unique_str("google");
    let login_data = json!({
        "email": test_email,
        "google_sub": test_google_sub,
        "name": 789
    });

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
