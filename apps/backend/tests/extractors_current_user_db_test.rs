mod common;
use common::assert_problem_details_structure;

use actix_web::{test, web};
use backend::{
    auth::mint_access_token,
    state::{AppState, SecurityConfig},
    test_support::{
        create_test_app, factories::seed_user_with_sub, get_test_db_url,
        schema_guard::ensure_schema_ready,
    },
};
use sea_orm::Database;
use serde_json::Value;
use std::time::SystemTime;

#[actix_web::test]
async fn test_me_db_success() {
    // Set up database
    let db_url = get_test_db_url();
    let db = Database::connect(&db_url)
        .await
        .expect("connect to test database");
    ensure_schema_ready(&db).await;

    // Create test security config and app state
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let app_state = AppState::new(db.clone(), security_config.clone());

    // Seed user with specific sub - use timestamp to ensure uniqueness
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let test_sub = format!("test-sub-{timestamp}");
    let test_email = format!("test-{timestamp}@example.com");
    let user = seed_user_with_sub(&db, &test_sub, Some(&test_email))
        .await
        .expect("should create user successfully");

    // Mint JWT with the same sub
    let token = mint_access_token(&test_sub, &test_email, SystemTime::now(), &security_config)
        .expect("should mint token successfully");

    let app = create_test_app(web::Data::new(app_state)).await;

    // Make request with valid token
    let req = test::TestRequest::get()
        .uri("/api/private/me_db")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 200
    assert_eq!(resp.status().as_u16(), 200);

    // Read and parse response body
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    let response: Value = serde_json::from_str(&body_str).expect("should parse JSON");

    // Verify response structure
    assert_eq!(response["id"], user.id);
    assert_eq!(response["sub"], test_sub);
    assert_eq!(response["email"], Value::Null); // email is None since it's not in users table
}

#[actix_web::test]
async fn test_me_db_user_not_found() {
    // Set up database
    let db_url = get_test_db_url();
    let db = Database::connect(&db_url)
        .await
        .expect("connect to test database");
    ensure_schema_ready(&db).await;

    // Create test security config and app state
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let app_state = AppState::new(db.clone(), security_config.clone());

    // Mint JWT with a sub that doesn't exist in database - use timestamp to ensure uniqueness
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let missing_sub = format!("missing-sub-{timestamp}");
    let test_email = format!("missing-{timestamp}@example.com");
    let token = mint_access_token(
        &missing_sub,
        &test_email,
        SystemTime::now(),
        &security_config,
    )
    .expect("should mint token successfully");

    let app = create_test_app(web::Data::new(app_state)).await;

    // Make request with valid token but non-existent user
    let req = test::TestRequest::get()
        .uri("/api/private/me_db")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 403
    assert_eq!(resp.status().as_u16(), 403);

    // Validate error structure
    assert_problem_details_structure(
        resp,
        403,
        "FORBIDDEN_USER_NOT_FOUND",
        "User not found in database",
    )
    .await;
}

#[actix_web::test]
async fn test_me_db_unauthorized() {
    // Set up database
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
    let req = test::TestRequest::get()
        .uri("/api/private/me_db")
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure
    assert_problem_details_structure(
        resp,
        401,
        "UNAUTHORIZED_MISSING_BEARER",
        "Missing or malformed Bearer token",
    )
    .await;
}
