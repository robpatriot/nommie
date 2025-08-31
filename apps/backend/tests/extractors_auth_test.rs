mod common;
use common::assert_problem_details_structure;

use actix_web::{test, App};
use backend::{
    auth::mint_access_token, extractors::BackendClaims, middleware::RequestTrace, routes::private,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Helper function to create a JWT token with custom claims
fn create_custom_jwt(claims: &BackendClaims, secret: &str) -> String {
    let jwt_claims = backend::extractors::JwtClaims {
        claims: claims.clone(),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &jwt_claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .expect("Failed to encode JWT")
}

/// Helper function to create expired JWT claims
fn create_expired_claims(sub: Uuid, email: &str) -> BackendClaims {
    let now = SystemTime::now();
    let iat = now
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get current time")
        .as_secs() as usize;

    // Set expiration to 1 hour ago
    let exp = iat - 3600;

    BackendClaims {
        sub,
        email: email.to_string(),
        exp,
    }
}

#[actix_web::test]
#[serial_test::serial]
async fn test_missing_header() {
    // Set up test environment
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .configure(private::configure_routes),
    )
    .await;

    // Make request without Authorization header
    let req = test::TestRequest::get().uri("/api/private/me").to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure - using current error codes/details
    assert_problem_details_structure(resp, 401, "UNAUTHORIZED", "Authentication required").await;

    // Clean up
    if let Some(secret) = original_secret {
        std::env::set_var("APP_JWT_SECRET", secret);
    } else {
        std::env::remove_var("APP_JWT_SECRET");
    }
}

#[actix_web::test]
#[serial_test::serial]
async fn test_malformed_scheme() {
    // Set up test environment
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .configure(private::configure_routes),
    )
    .await;

    // Test malformed Authorization header
    let req = test::TestRequest::get()
        .uri("/api/private/me")
        .insert_header(("Authorization", "Token abc"))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure - using current error codes/details
    assert_problem_details_structure(resp, 401, "UNAUTHORIZED", "Authentication required").await;

    // Clean up
    if let Some(secret) = original_secret {
        std::env::set_var("APP_JWT_SECRET", secret);
    } else {
        std::env::remove_var("APP_JWT_SECRET");
    }
}

#[actix_web::test]
#[serial_test::serial]
async fn test_empty_token() {
    // Set up test environment
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .configure(private::configure_routes),
    )
    .await;

    // Test empty token
    let req = test::TestRequest::get()
        .uri("/api/private/me")
        .insert_header(("Authorization", "Bearer "))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure - using current error codes/details
    assert_problem_details_structure(resp, 401, "UNAUTHORIZED", "Authentication required").await;

    // Clean up
    if let Some(secret) = original_secret {
        std::env::set_var("APP_JWT_SECRET", secret);
    } else {
        std::env::remove_var("APP_JWT_SECRET");
    }
}

#[actix_web::test]
#[serial_test::serial]
async fn test_invalid_token() {
    // Set up test environment
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .configure(private::configure_routes),
    )
    .await;

    // Test with invalid token
    let req = test::TestRequest::get()
        .uri("/api/private/me")
        .insert_header(("Authorization", "Bearer not-a-real-token"))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure - using current error codes/details
    assert_problem_details_structure(resp, 401, "UNAUTHORIZED", "Authentication required").await;

    // Clean up
    if let Some(secret) = original_secret {
        std::env::set_var("APP_JWT_SECRET", secret);
    } else {
        std::env::remove_var("APP_JWT_SECRET");
    }
}

#[actix_web::test]
#[serial_test::serial]
async fn test_expired_token() {
    // Set up test environment
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .configure(private::configure_routes),
    )
    .await;

    // Create expired JWT claims
    let sub = Uuid::new_v4();
    let email = "test@example.com";
    let expired_claims = create_expired_claims(sub, email);
    let expired_token =
        create_custom_jwt(&expired_claims, "test_secret_key_for_testing_purposes_only");

    // Test with expired token
    let req = test::TestRequest::get()
        .uri("/api/private/me")
        .insert_header(("Authorization", format!("Bearer {expired_token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure - using current error codes/details
    assert_problem_details_structure(resp, 401, "UNAUTHORIZED", "Authentication required").await;

    // Clean up
    if let Some(secret) = original_secret {
        std::env::set_var("APP_JWT_SECRET", secret);
    } else {
        std::env::remove_var("APP_JWT_SECRET");
    }
}

#[actix_web::test]
#[serial_test::serial]
async fn test_happy_path() {
    // Set up test environment
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .configure(private::configure_routes),
    )
    .await;

    // Create a valid JWT token
    let sub = Uuid::new_v4();
    let email = "test@example.com";
    let token = mint_access_token(sub, email, SystemTime::now()).unwrap();

    // Make request with valid token
    let req = test::TestRequest::get()
        .uri("/api/private/me")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert success
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["sub"], sub.to_string());
    assert_eq!(body["email"], email);

    // Clean up
    if let Some(secret) = original_secret {
        std::env::set_var("APP_JWT_SECRET", secret);
    } else {
        std::env::remove_var("APP_JWT_SECRET");
    }
}
