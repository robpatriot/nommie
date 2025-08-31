use actix_web::{test, web, App};
use backend::{
    auth::mint_access_token, extractors::BackendAuth, middleware::RequestTrace, AppError,
};
use serde_json::Value;
use std::time::SystemTime;
use uuid::Uuid;

// Helper function to validate that a response follows the ProblemDetails structure
async fn assert_problem_details_structure(
    resp: actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
    expected_status: u16,
    expected_code: &str,
    expected_detail: &str,
) {
    // Assert status code
    assert_eq!(resp.status().as_u16(), expected_status);

    // Extract headers before consuming the response
    let headers = resp.headers().clone();
    let request_id_header = headers.get("x-request-id");
    assert!(
        request_id_header.is_some(),
        "X-Request-Id header should be present"
    );
    let request_id = request_id_header.unwrap().to_str().unwrap();
    assert!(
        !request_id.is_empty(),
        "X-Request-Id header should not be empty"
    );

    // Assert Content-Type is application/problem+json
    let content_type = headers.get("content-type").unwrap().to_str().unwrap();
    assert_eq!(content_type, "application/problem+json");

    // Read and parse the response body
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Improved error handling for deserialization failures with more descriptive error message
    let problem_details: serde_json::Value = match serde_json::from_str(&body_str) {
        Ok(details) => details,
        Err(_) => panic!("Failed to parse error body as ProblemDetails. Raw body: {body_str}"),
    };

    // Assert all required keys are present
    assert!(
        problem_details.get("type").is_some(),
        "type field should be present"
    );
    assert!(
        problem_details.get("title").is_some(),
        "title field should be present"
    );
    assert!(
        problem_details.get("status").is_some(),
        "status field should be present"
    );
    assert!(
        problem_details.get("detail").is_some(),
        "detail field should be present"
    );
    assert!(
        problem_details.get("code").is_some(),
        "code field should be present"
    );
    assert!(
        problem_details.get("trace_id").is_some(),
        "trace_id field should be present"
    );

    // Assert specific values
    assert_eq!(problem_details["code"], expected_code);
    assert_eq!(problem_details["detail"], expected_detail);
    assert_eq!(problem_details["status"], expected_status);

    // Assert trace_id matches the X-Request-Id header
    let trace_id_in_body = problem_details["trace_id"].as_str().unwrap();
    assert_eq!(
        trace_id_in_body, request_id,
        "trace_id in body should match X-Request-Id header"
    );

    // Assert type follows the expected format
    let type_value = problem_details["type"].as_str().unwrap();
    assert!(
        type_value.starts_with("https://nommie.app/errors/"),
        "type should follow the expected URL format"
    );
}

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
