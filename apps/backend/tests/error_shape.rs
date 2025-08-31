mod common;
use common::assert_problem_details_structure;

use actix_web::{test, web, App, HttpMessage, HttpRequest, HttpResponse};
use backend::{middleware::RequestTrace, AppError};
use serde_json::Value;

/// Test endpoint that returns a validation error (400)
async fn test_validation_error(req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(
        AppError::invalid("VALIDATION_ERROR", "Field validation failed".to_string())
            .with_trace_id(req.extensions().get::<String>().cloned()),
    )
}

/// Test endpoint that returns a bad request error (400)
async fn test_bad_request_error(req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(
        AppError::bad_request("BAD_REQUEST", "Invalid request format".to_string())
            .with_trace_id(req.extensions().get::<String>().cloned()),
    )
}

/// Test endpoint that returns a not found error (404)
async fn test_not_found_error(req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(
        AppError::not_found("NOT_FOUND", "Resource not found".to_string())
            .with_trace_id(req.extensions().get::<String>().cloned()),
    )
}

/// Test endpoint that returns an unauthorized error (401)
async fn test_unauthorized_error(req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::unauthorized().with_trace_id(req.extensions().get::<String>().cloned()))
}

/// Test endpoint that returns a forbidden error (403)
async fn test_forbidden_error(req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::forbidden().with_trace_id(req.extensions().get::<String>().cloned()))
}

/// Test endpoint that returns an internal server error (500)
async fn test_internal_error(req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::internal("Database connection failed".to_string())
        .with_trace_id(req.extensions().get::<String>().cloned()))
}

/// Test endpoint that returns a database error (500)
async fn test_db_error(req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::db("Connection timeout".to_string())
        .with_trace_id(req.extensions().get::<String>().cloned()))
}

/// Test that all error responses conform to ProblemDetails format
/// This test consolidates all error type testing into a single, parameterized test
#[actix_web::test]
async fn test_all_error_responses_conform_to_problem_details() {
    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .route("/_test/validation", web::get().to(test_validation_error))
            .route("/_test/bad_request", web::get().to(test_bad_request_error))
            .route("/_test/not_found", web::get().to(test_not_found_error))
            .route(
                "/_test/unauthorized",
                web::get().to(test_unauthorized_error),
            )
            .route("/_test/forbidden", web::get().to(test_forbidden_error))
            .route("/_test/internal", web::get().to(test_internal_error))
            .route("/_test/db", web::get().to(test_db_error)),
    )
    .await;

    // Test all error types to ensure they conform to ProblemDetails format
    let error_cases = vec![
        (
            "/_test/validation",
            400,
            "VALIDATION_ERROR",
            "Field validation failed",
        ),
        (
            "/_test/bad_request",
            400,
            "BAD_REQUEST",
            "Invalid request format",
        ),
        ("/_test/not_found", 404, "NOT_FOUND", "Resource not found"),
        (
            "/_test/unauthorized",
            401,
            "UNAUTHORIZED",
            "Authentication required",
        ),
        ("/_test/forbidden", 403, "FORBIDDEN", "Access denied"),
        (
            "/_test/internal",
            500,
            "INTERNAL",
            "Database connection failed",
        ),
        ("/_test/db", 500, "DB_ERROR", "Connection timeout"),
    ];

    for (endpoint, status, code, detail) in error_cases {
        let req = test::TestRequest::get().uri(endpoint).to_request();
        let resp = test::call_service(&app, req).await;
        assert_problem_details_structure(resp, status, code, detail).await;
    }
}

/// Test that successful responses don't interfere with error handling
#[actix_web::test]
async fn test_successful_response_with_error_handling() {
    async fn success_handler() -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().body("Success"))
    }

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .route("/_test/success", web::get().to(success_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/_test/success").to_request();
    let resp = test::call_service(&app, req).await;

    // Successful response should have 200 status
    assert_eq!(resp.status().as_u16(), 200);

    // Should still have X-Request-Id header
    let headers = resp.headers();
    let request_id_header = headers.get("x-request-id");
    assert!(
        request_id_header.is_some(),
        "X-Request-Id header should be present on successful responses"
    );

    // Body should be the success message
    let body = test::read_body(resp).await;
    assert_eq!(body, "Success");
}

/// Test edge case: error with missing trace_id (should fallback to "unknown")
#[actix_web::test]
async fn test_error_without_trace_id() {
    async fn error_without_trace() -> Result<HttpResponse, AppError> {
        // Create error without trace_id and use the centralized function
        Err(AppError::ensure_trace_id(AppError::invalid(
            "NO_TRACE",
            "Error without trace".to_string(),
        )))
    }

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .route("/_test/no_trace", web::get().to(error_without_trace)),
    )
    .await;

    let req = test::TestRequest::get().uri("/_test/no_trace").to_request();
    let resp = test::call_service(&app, req).await;

    // Should still return 400 status
    assert_eq!(resp.status().as_u16(), 400);

    // Should have X-Request-Id header from middleware
    let headers = resp.headers();
    let request_id_header = headers.get("x-request-id");
    assert!(
        request_id_header.is_some(),
        "X-Request-Id header should be present"
    );

    // Body should have trace_id as "unknown" since it wasn't set
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    let problem_details: Value = serde_json::from_str(&body_str).unwrap();

    assert_eq!(problem_details["trace_id"], "unknown");
    assert_eq!(problem_details["code"], "NO_TRACE");
    assert_eq!(problem_details["detail"], "Error without trace");
}

/// Test that the centralized ensure_trace_id function works correctly
#[actix_web::test]
async fn test_ensure_trace_id_function() {
    // Test with None trace_id
    let error = AppError::invalid("TEST", "Test error".to_string());
    let error_with_trace = AppError::ensure_trace_id(error);

    match error_with_trace {
        AppError::Validation { trace_id, .. } => {
            assert_eq!(trace_id, Some("unknown".to_string()));
        }
        _ => panic!("Expected Validation error variant"),
    }

    // Test with Some trace_id
    let error = AppError::invalid("TEST", "Test error".to_string())
        .with_trace_id(Some("custom-trace".to_string()));
    let error_with_trace = AppError::ensure_trace_id(error);

    match error_with_trace {
        AppError::Validation { trace_id, .. } => {
            assert_eq!(trace_id, Some("custom-trace".to_string()));
        }
        _ => panic!("Expected Validation error variant"),
    }
}

/// Test edge case: malformed error response handling
#[actix_web::test]
async fn test_malformed_error_response_handling() {
    async fn malformed_error() -> Result<HttpResponse, AppError> {
        // This would create a malformed response if not handled properly
        Err(AppError::internal("Malformed error test".to_string())
            .with_trace_id(Some("test-trace".to_string())))
    }

    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .route("/_test/malformed", web::get().to(malformed_error)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/_test/malformed")
        .to_request();
    let resp = test::call_service(&app, req).await;

    // Should return 500 status
    assert_eq!(resp.status().as_u16(), 500);

    // Should have proper headers
    let headers = resp.headers();
    let request_id_header = headers.get("x-request-id");
    assert!(
        request_id_header.is_some(),
        "X-Request-Id header should be present"
    );

    let content_type = headers.get("content-type").unwrap().to_str().unwrap();
    assert_eq!(content_type, "application/problem+json");

    // Body should be valid JSON with all required fields
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // This should not panic and should parse correctly
    let problem_details: Value = serde_json::from_str(&body_str).unwrap();

    // Verify all required fields are present
    assert!(problem_details.get("type").is_some());
    assert!(problem_details.get("title").is_some());
    assert!(problem_details.get("status").is_some());
    assert!(problem_details.get("detail").is_some());
    assert!(problem_details.get("code").is_some());
    assert!(problem_details.get("trace_id").is_some());

    // Verify specific values
    assert_eq!(problem_details["status"], 500);
    assert_eq!(problem_details["code"], "INTERNAL");
    assert_eq!(problem_details["detail"], "Malformed error test");
}
