mod common;
mod support;
use actix_web::{test, web, HttpRequest, HttpResponse};
use backend::config::db::DbProfile;
use backend::infra::state::build_state;
use backend::AppError;
use common::assert_problem_details_structure;
use serde_json::Value;
use support::create_test_app;

/// Test endpoint that returns a validation error (400)
async fn test_validation_error(_req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::invalid(
        "VALIDATION_ERROR",
        "Field validation failed".to_string(),
    ))
}

/// Test endpoint that returns a bad request error (400)
async fn test_bad_request_error(_req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::bad_request(
        "BAD_REQUEST",
        "Invalid request format".to_string(),
    ))
}

/// Test endpoint that returns a not found error (404)
async fn test_not_found_error(_req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::not_found(
        "NOT_FOUND",
        "Resource not found".to_string(),
    ))
}

/// Test endpoint that returns an unauthorized error (401)
async fn test_unauthorized_error(_req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::unauthorized())
}

/// Test endpoint that returns a forbidden error (403)
async fn test_forbidden_error(_req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::forbidden())
}

/// Test endpoint that returns an internal server error (500)
async fn test_internal_error(_req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::internal("Database connection failed".to_string()))
}

/// Test endpoint that returns a database error (500)
async fn test_db_error(_req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::db("Connection timeout".to_string()))
}

/// Test that all error responses conform to ProblemDetails format
/// This test consolidates all error type testing into a single, parameterized test
#[actix_web::test]
async fn test_all_error_responses_conform_to_problem_details() {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("create test state");
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route("/_test/validation", web::get().to(test_validation_error))
                .route("/_test/bad_request", web::get().to(test_bad_request_error))
                .route("/_test/not_found", web::get().to(test_not_found_error))
                .route(
                    "/_test/unauthorized",
                    web::get().to(test_unauthorized_error),
                )
                .route("/_test/forbidden", web::get().to(test_forbidden_error))
                .route("/_test/internal", web::get().to(test_internal_error))
                .route("/_test/db", web::get().to(test_db_error));
        })
        .build()
        .await
        .expect("create test app");

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

    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("create test state");
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route("/_test/success", web::get().to(success_handler));
        })
        .build()
        .await
        .expect("create test app");

    let req = test::TestRequest::get().uri("/_test/success").to_request();
    let resp = test::call_service(&app, req).await;

    // Successful response should have 200 status
    assert_eq!(resp.status().as_u16(), 200);

    // Should still have X-Trace-Id header
    let headers = resp.headers();
    let trace_id_header = headers.get("x-trace-id");
    assert!(
        trace_id_header.is_some(),
        "X-Trace-Id header should be present on successful responses"
    );

    // Body should be the success message
    let body = test::read_body(resp).await;
    assert_eq!(body, "Success");
}

/// Test edge case: error with trace_id from task-local context
#[actix_web::test]
async fn test_error_with_trace_id_from_context() {
    async fn error_with_trace() -> Result<HttpResponse, AppError> {
        // Create error - trace_id will come from task-local context
        Err(AppError::invalid(
            "NO_TRACE",
            "Error without trace".to_string(),
        ))
    }

    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("create test state");
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route("/_test/no_trace", web::get().to(error_with_trace));
        })
        .build()
        .await
        .expect("create test app");

    let req = test::TestRequest::get().uri("/_test/no_trace").to_request();
    let resp = test::call_service(&app, req).await;

    // Should still return 400 status
    assert_eq!(resp.status().as_u16(), 400);

    // Should have X-Trace-Id header from middleware
    let headers = resp.headers();
    let trace_id_header = headers.get("x-trace-id");
    assert!(
        trace_id_header.is_some(),
        "X-Trace-Id header should be present"
    );

    let header_trace_id = trace_id_header.unwrap().to_str().unwrap().to_string();

    // Body should have trace_id matching the header
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    let problem_details: Value = serde_json::from_str(&body_str).unwrap();

    assert_eq!(problem_details["trace_id"], header_trace_id);
    assert_eq!(problem_details["code"], "NO_TRACE");
    assert_eq!(problem_details["detail"], "Error without trace");
}

/// Test that trace_ctx::trace_id() returns "unknown" outside of request context
#[actix_web::test]
async fn test_trace_ctx_outside_context() {
    use backend::web::trace_ctx;

    // Outside of a request context, should return "unknown"
    assert_eq!(trace_ctx::trace_id(), "unknown");
}

/// Test edge case: malformed error response handling
#[actix_web::test]
async fn test_malformed_error_response_handling() {
    async fn malformed_error() -> Result<HttpResponse, AppError> {
        // This would create a malformed response if not handled properly
        Err(AppError::internal("Malformed error test".to_string()))
    }

    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("create test state");
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route("/_test/malformed", web::get().to(malformed_error));
        })
        .build()
        .await
        .expect("create test app");

    let req = test::TestRequest::get()
        .uri("/_test/malformed")
        .to_request();
    let resp = test::call_service(&app, req).await;

    // Should return 500 status
    assert_eq!(resp.status().as_u16(), 500);

    // Should have proper headers
    let headers = resp.headers();
    let trace_id_header = headers.get("x-trace-id");
    assert!(
        trace_id_header.is_some(),
        "X-Trace-Id header should be present"
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
