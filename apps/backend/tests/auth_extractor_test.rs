mod common;
use common::assert_problem_details_structure;

use actix_web::{test, web, App};
use backend::{
    auth::mint_access_token, extractors::BackendAuth, middleware::RequestTrace, AppError,
};
use serde_json::Value;
use std::time::SystemTime;
use uuid::Uuid;

/// Test endpoint that uses the BackendAuth extractor
async fn test_protected_endpoint(auth: BackendAuth) -> Result<web::Json<Value>, AppError> {
    let response = serde_json::json!({
        "sub": auth.sub,
        "email": auth.email
    });
    Ok(web::Json(response))
}

#[actix_web::test]
#[serial_test::serial]
async fn test_backend_auth_extractor_success() {
    // Set up test environment
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .service(web::resource("/test").to(test_protected_endpoint)),
    )
    .await;

    // Create a valid JWT token
    let user_id = Uuid::new_v4();
    let email = "test@example.com";
    let token = mint_access_token(user_id, email, SystemTime::now()).unwrap();

    // Make request with valid token
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert success
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["sub"], user_id.to_string());
    assert_eq!(body["email"], email);

    // Clean up
    if let Some(secret) = original_secret {
        std::env::set_var("APP_JWT_SECRET", secret);
    } else {
        std::env::remove_var("APP_JWT_SECRET");
    }
}

#[actix_web::test]
#[serial_test::serial]
async fn test_backend_auth_extractor_missing_header() {
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .service(web::resource("/test").to(test_protected_endpoint)),
    )
    .await;

    // Make request without Authorization header
    let req = test::TestRequest::get().uri("/test").to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure
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
async fn test_backend_auth_extractor_malformed_header() {
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .service(web::resource("/test").to(test_protected_endpoint)),
    )
    .await;

    // Test various malformed headers
    let malformed_headers = vec![
        "Token abc123",
        "Bearer",
        "Bearer ",
        "Basic abc123",
        "abc123",
    ];

    for header_value in malformed_headers {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", header_value))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert 401
        assert_eq!(resp.status().as_u16(), 401);

        // Validate error structure
        assert_problem_details_structure(resp, 401, "UNAUTHORIZED", "Authentication required")
            .await;
    }

    // Clean up
    if let Some(secret) = original_secret {
        std::env::set_var("APP_JWT_SECRET", secret);
    } else {
        std::env::remove_var("APP_JWT_SECRET");
    }
}

#[actix_web::test]
#[serial_test::serial]
async fn test_backend_auth_extractor_invalid_token() {
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .service(web::resource("/test").to(test_protected_endpoint)),
    )
    .await;

    // Test with invalid tokens
    let invalid_tokens = vec![
        "invalid.jwt.token",
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.invalid_signature",
        "not_even_close_to_jwt",
    ];

    for token in invalid_tokens {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert 401
        assert_eq!(resp.status().as_u16(), 401);

        // Validate error structure
        assert_problem_details_structure(resp, 401, "UNAUTHORIZED", "Authentication required")
            .await;
    }

    // Clean up
    if let Some(secret) = original_secret {
        std::env::set_var("APP_JWT_SECRET", secret);
    } else {
        std::env::remove_var("APP_JWT_SECRET");
    }
}

#[actix_web::test]
#[serial_test::serial]
async fn test_backend_auth_extractor_expired_token() {
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .service(web::resource("/test").to(test_protected_endpoint)),
    )
    .await;

    // Create an expired token (20 minutes ago, but token expires in 15 minutes)
    let user_id = Uuid::new_v4();
    let email = "test@example.com";
    let expired_time = SystemTime::now() - std::time::Duration::from_secs(20 * 60);
    let expired_token = mint_access_token(user_id, email, expired_time).unwrap();

    // Make request with expired token
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {expired_token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure
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
async fn test_backend_auth_extractor_wrong_secret() {
    let original_secret = std::env::var("APP_JWT_SECRET").ok();

    // Create token with one secret
    std::env::set_var("APP_JWT_SECRET", "secret_a");

    let user_id = Uuid::new_v4();
    let email = "test@example.com";
    let token = mint_access_token(user_id, email, SystemTime::now()).unwrap();

    // Verify with different secret
    std::env::set_var("APP_JWT_SECRET", "secret_b");

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .service(web::resource("/test").to(test_protected_endpoint)),
    )
    .await;

    // Make request with token signed with different secret
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure
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
async fn test_protected_me_endpoint_success() {
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .configure(backend::routes::configure),
    )
    .await;

    // Create a valid JWT token
    let user_id = Uuid::new_v4();
    let email = "test@example.com";
    let token = mint_access_token(user_id, email, SystemTime::now()).unwrap();

    // Make request to the actual protected endpoint
    let req = test::TestRequest::get()
        .uri("/api/private/me")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert success
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["sub"], user_id.to_string());
    assert_eq!(body["email"], email);

    // Clean up
    if let Some(secret) = original_secret {
        std::env::set_var("APP_JWT_SECRET", secret);
    } else {
        std::env::remove_var("APP_JWT_SECRET");
    }
}

#[actix_web::test]
#[serial_test::serial]
async fn test_protected_me_endpoint_unauthorized() {
    let original_secret = std::env::var("APP_JWT_SECRET").ok();
    std::env::set_var(
        "APP_JWT_SECRET",
        "test_secret_key_for_testing_purposes_only",
    );

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .configure(backend::routes::configure),
    )
    .await;

    // Make request without Authorization header
    let req = test::TestRequest::get().uri("/api/private/me").to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure
    assert_problem_details_structure(resp, 401, "UNAUTHORIZED", "Authentication required").await;

    // Clean up
    if let Some(secret) = original_secret {
        std::env::set_var("APP_JWT_SECRET", secret);
    } else {
        std::env::remove_var("APP_JWT_SECRET");
    }
}
