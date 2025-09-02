mod common;
use common::assert_problem_details_structure;

use actix_web::test;
use backend::{
    auth::mint_access_token,
    state::SecurityConfig,
    test_support::{create_test_app, create_test_state},
};
use serde_json::Value;
use std::time::SystemTime;

#[actix_web::test]
async fn test_missing_header() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and default security config
    let state = create_test_state().with_db().build().await?;

    // Build app with production routes
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

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

    Ok(())
}

#[actix_web::test]
async fn test_malformed_scheme() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and default security config
    let state = create_test_state().with_db().build().await?;

    // Build app with production routes
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

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

    Ok(())
}

#[actix_web::test]
async fn test_empty_token() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and default security config
    let state = create_test_state().with_db().build().await?;

    // Build app with production routes
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

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

    Ok(())
}

#[actix_web::test]
async fn test_invalid_token() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and default security config
    let state = create_test_state().with_db().build().await?;

    // Build app with production routes
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

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

    Ok(())
}

#[actix_web::test]
async fn test_expired_token() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and custom security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = create_test_state()
        .with_db()
        .with_security(security_config.clone())
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

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

    Ok(())
}

#[actix_web::test]
async fn test_happy_path() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and custom security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = create_test_state()
        .with_db()
        .with_security(security_config.clone())
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

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

    Ok(())
}
