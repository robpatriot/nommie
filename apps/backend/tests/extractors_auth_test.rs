mod common;
use common::assert_problem_details_structure;

use actix_web::{test, web};
use backend::{
    auth::mint_access_token,
    state::{AppState, SecurityConfig},
    test_support::{create_test_app, get_test_db_url, schema_guard::ensure_schema_ready},
};
use sea_orm::Database;
use serde_json::Value;
use std::time::SystemTime;

#[actix_web::test]
async fn test_missing_header() {
    // Set up test database
    let db_url = get_test_db_url();
    let db = Database::connect(&db_url)
        .await
        .expect("connect to test database");
    ensure_schema_ready(&db).await;

    // Create test security config and app state
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let app_state = AppState::new(db, security_config);

    let app = create_test_app(web::Data::new(app_state)).await;

    // Make request without Authorization header
    let req = test::TestRequest::get().uri("/api/private/me").to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure - using current error codes/details
    assert_problem_details_structure(
        resp,
        401,
        "UNAUTHORIZED_MISSING_BEARER",
        "Missing or malformed Bearer token",
    )
    .await;
}

#[actix_web::test]
async fn test_malformed_scheme() {
    // Set up test database
    let db_url = get_test_db_url();
    let db = Database::connect(&db_url)
        .await
        .expect("connect to test database");
    ensure_schema_ready(&db).await;

    // Create test security config and app state
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let app_state = AppState::new(db, security_config);

    let app = create_test_app(web::Data::new(app_state)).await;

    // Test malformed Authorization header
    let req = test::TestRequest::get()
        .uri("/api/private/me")
        .insert_header(("Authorization", "Token abc"))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure - using current error codes/details
    assert_problem_details_structure(
        resp,
        401,
        "UNAUTHORIZED_MISSING_BEARER",
        "Missing or malformed Bearer token",
    )
    .await;
}

#[actix_web::test]
async fn test_empty_token() {
    // Set up test database
    let db_url = get_test_db_url();
    let db = Database::connect(&db_url)
        .await
        .expect("connect to test database");
    ensure_schema_ready(&db).await;

    // Create test security config and app state
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let app_state = AppState::new(db, security_config);

    let app = create_test_app(web::Data::new(app_state)).await;

    // Test empty token
    let req = test::TestRequest::get()
        .uri("/api/private/me")
        .insert_header(("Authorization", "Bearer "))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure - using current error codes/details
    assert_problem_details_structure(
        resp,
        401,
        "UNAUTHORIZED_MISSING_BEARER",
        "Missing or malformed Bearer token",
    )
    .await;
}

#[actix_web::test]
async fn test_invalid_token() {
    // Set up test database
    let db_url = get_test_db_url();
    let db = Database::connect(&db_url)
        .await
        .expect("connect to test database");
    ensure_schema_ready(&db).await;

    // Create test security config and app state
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let app_state = AppState::new(db, security_config);

    let app = create_test_app(web::Data::new(app_state)).await;

    // Test with invalid token
    let req = test::TestRequest::get()
        .uri("/api/private/me")
        .insert_header(("Authorization", "Bearer not-a-real-token"))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure - using current error codes/details
    assert_problem_details_structure(resp, 401, "UNAUTHORIZED_INVALID_JWT", "Invalid JWT").await;
}

#[actix_web::test]
async fn test_expired_token() {
    // Set up test database
    let db_url = get_test_db_url();
    let db = Database::connect(&db_url)
        .await
        .expect("connect to test database");
    ensure_schema_ready(&db).await;

    // Create test security config and app state
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let app_state = AppState::new(db, security_config.clone());

    let app = create_test_app(web::Data::new(app_state)).await;

    // Create expired JWT token by using a time from the past
    let sub = "test-sub-expired-123";
    let email = "test@example.com";
    let past_time = SystemTime::now() - std::time::Duration::from_secs(20 * 60); // 20 minutes ago
    let expired_token = mint_access_token(sub, email, past_time, &security_config).unwrap();

    // Test with expired token
    let req = test::TestRequest::get()
        .uri("/api/private/me")
        .insert_header(("Authorization", format!("Bearer {expired_token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure - using current error codes/details
    assert_problem_details_structure(resp, 401, "UNAUTHORIZED_EXPIRED_JWT", "Token expired").await;
}

#[actix_web::test]
async fn test_happy_path() {
    // Set up test database
    let db_url = get_test_db_url();
    let db = Database::connect(&db_url)
        .await
        .expect("connect to test database");
    ensure_schema_ready(&db).await;

    // Create test security config and app state
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let app_state = AppState::new(db, security_config.clone());

    let app = create_test_app(web::Data::new(app_state)).await;

    // Create a valid JWT token
    let sub = "test-sub-happy-456";
    let email = "test@example.com";
    let token = mint_access_token(sub, email, SystemTime::now(), &security_config).unwrap();

    // Make request with valid token
    let req = test::TestRequest::get()
        .uri("/api/private/me")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert success
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["sub"], sub);
    assert_eq!(body["email"], email);
}
