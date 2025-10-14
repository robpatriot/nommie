use actix_web::{test, web, HttpResponse};
use backend::config::db::DbProfile;
use backend::infra::state::build_state;
use backend::state::app_state::AppState;
use backend::{AppError, ErrorCode};

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;

/// Test endpoint that returns a validation error (400)
async fn test_validation_error() -> Result<HttpResponse, AppError> {
    Err(AppError::invalid(
        ErrorCode::ValidationError,
        "Field validation failed".to_string(),
    ))
}

/// Test endpoint that returns a bad request error (400)
async fn test_bad_request_error() -> Result<HttpResponse, AppError> {
    Err(AppError::bad_request(
        ErrorCode::BadRequest,
        "Invalid request format".to_string(),
    ))
}

/// Test endpoint that returns a not found error (404)
async fn test_not_found_error() -> Result<HttpResponse, AppError> {
    Err(AppError::not_found(
        ErrorCode::NotFound,
        "Resource not found".to_string(),
    ))
}

/// Test endpoint that returns an unauthorized error (401)
async fn test_unauthorized_error() -> Result<HttpResponse, AppError> {
    Err(AppError::unauthorized())
}

/// Test endpoint that returns a forbidden error (403)
async fn test_forbidden_error() -> Result<HttpResponse, AppError> {
    Err(AppError::forbidden())
}

/// Test endpoint that returns an internal server error (500)
async fn test_internal_error() -> Result<HttpResponse, AppError> {
    Err(AppError::internal("Database connection failed"))
}

/// Test endpoint that returns a database error (500)
async fn test_db_error() -> Result<HttpResponse, AppError> {
    Err(AppError::db("Connection timeout"))
}

/// Test endpoint that returns a database unavailable error (500)
async fn test_db_unavailable_error() -> Result<HttpResponse, AppError> {
    Err(AppError::db_unavailable())
}

// handler-only: validates error shape; no DB
/// Test that all error responses conform to ProblemDetails format
/// This test consolidates all error type testing into a single, parameterized test
#[actix_web::test]
async fn test_all_error_responses_conform_to_problem_details() {
    let state = build_state().build().await.expect("create test state");
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
                .route("/_test/db", web::get().to(test_db_error))
                .route(
                    "/_test/db_unavailable",
                    web::get().to(test_db_unavailable_error),
                );
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
            "INTERNAL_ERROR",
            "Database connection failed",
        ),
        ("/_test/db", 500, "DB_ERROR", "Connection timeout"),
        (
            "/_test/db_unavailable",
            503,
            "DB_UNAVAILABLE",
            "Database unavailable",
        ),
    ];

    for (endpoint, status, code, detail) in error_cases {
        let req = test::TestRequest::get().uri(endpoint).to_request();
        let resp = test::call_service(&app, req).await;
        assert_problem_details_structure(resp, status, code, detail).await;
    }
}

// handler-only: validates error shape; no DB
/// Test that successful responses don't interfere with error handling
#[actix_web::test]
async fn test_successful_response_with_error_handling() {
    async fn success_handler() -> Result<HttpResponse, AppError> {
        Ok(HttpResponse::Ok().body("Success"))
    }

    let state = build_state().build().await.expect("create test state");
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

// handler-only: validates error shape; no DB
/// Test edge case: error with trace_id from task-local context
#[actix_web::test]
async fn test_error_with_trace_id_from_context() {
    async fn error_with_trace() -> Result<HttpResponse, AppError> {
        // Create error - trace_id will come from task-local context
        Err(AppError::invalid(
            ErrorCode::NoTrace,
            "Error without trace".to_string(),
        ))
    }

    let state = build_state().build().await.expect("create test state");
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route("/_test/no_trace", web::get().to(error_with_trace));
        })
        .build()
        .await
        .expect("create test app");

    let req = test::TestRequest::get().uri("/_test/no_trace").to_request();
    let resp = test::call_service(&app, req).await;

    // Validate error structure using centralized helper
    assert_problem_details_structure(resp, 400, "NO_TRACE", "Error without trace").await;
}

// handler-only: validates error shape; no DB
/// Test that trace_ctx::trace_id() returns "unknown" outside of request context
#[actix_web::test]
async fn test_trace_ctx_outside_context() {
    use backend::web::trace_ctx;

    // Outside of a request context, should return "unknown"
    assert_eq!(trace_ctx::trace_id(), "unknown");
}

// handler-only: validates error shape; no DB
/// Test edge case: malformed error response handling
#[actix_web::test]
async fn test_malformed_error_response_handling() {
    async fn malformed_error() -> Result<HttpResponse, AppError> {
        // This would create a malformed response if not handled properly
        Err(AppError::internal("Malformed error test"))
    }

    let state = build_state().build().await.expect("create test state");
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

    // Validate error structure using centralized helper
    assert_problem_details_structure(resp, 500, "INTERNAL_ERROR", "Malformed error test").await;
}

/// Test endpoint that uses require_db helper
async fn test_require_db_endpoint(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    use backend::db::require_db;

    // This will return DB_UNAVAILABLE if no DB is configured
    let _db = require_db(&state)?;

    // If DB exists, succeed
    Ok(HttpResponse::Ok().body("Success"))
}

// require_db: negative (no DB configured)
/// Test that require_db helper returns DB_UNAVAILABLE error when no DB is configured
#[actix_web::test]
async fn test_require_db_direct_without_database() {
    use backend::db::require_db;

    let state = build_state()
        .build() // build with no DB
        .await
        .expect("create test state without DB");

    let res = require_db(&state);
    assert!(
        res.is_err(),
        "require_db should fail when no DB is configured"
    );

    let err = res.unwrap_err();
    assert_eq!(err.code(), ErrorCode::DbUnavailable);
}

// require_db: negative (no DB configured)
/// Test that require_db helper returns DB_UNAVAILABLE error when no DB is configured
#[actix_web::test]
async fn test_require_db_without_database() {
    let state = build_state()
        .build() // build with no DB
        .await
        .expect("create test state without DB");

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route("/_test/require_db", web::get().to(test_require_db_endpoint));
        })
        .build()
        .await
        .expect("create test app");

    let req = test::TestRequest::get()
        .uri("/_test/require_db")
        .to_request();
    let resp = test::call_service(&app, req).await;

    // Should return DB_UNAVAILABLE problem details with trace id
    assert_problem_details_structure(resp, 503, "DB_UNAVAILABLE", "Database unavailable").await;
}

// require_db: positive (DB configured)
/// Test that require_db helper succeeds when DB is configured
#[actix_web::test]
async fn test_require_db_with_database() {
    let state = build_state()
        .with_db(DbProfile::Test) // build with Test DB
        .build()
        .await
        .expect("create test state with DB");

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route("/_test/require_db", web::get().to(test_require_db_endpoint));
        })
        .build()
        .await
        .expect("create test app");

    let req = test::TestRequest::get()
        .uri("/_test/require_db")
        .to_request();
    let resp = test::call_service(&app, req).await;

    // Should succeed with 200 OK
    assert_eq!(resp.status().as_u16(), 200);

    // Still should have X-Trace-Id header
    let headers = resp.headers();
    assert!(headers.get("x-trace-id").is_some());

    // Body should be "Success"
    let body = test::read_body(resp).await;
    assert_eq!(body, "Success");
}

/// Test endpoint that returns an optimistic lock conflict with extensions
async fn test_optimistic_lock_error() -> Result<HttpResponse, AppError> {
    Err(AppError::conflict_with_extensions(
        ErrorCode::OptimisticLock,
        "Resource was modified concurrently (expected version 5, actual version 7). Please refresh and retry.",
        serde_json::json!({ "expected": 5, "actual": 7 }),
    ))
}

/// Test that optimistic lock conflicts include extensions with version info
#[actix_web::test]
async fn test_optimistic_lock_extensions() {
    let state = build_state().build().await.expect("create test state");
    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.route(
                "/_test/optimistic_lock",
                web::get().to(test_optimistic_lock_error),
            );
        })
        .build()
        .await
        .expect("create test app");

    let req = test::TestRequest::get()
        .uri("/_test/optimistic_lock")
        .to_request();
    let resp = test::call_service(&app, req).await;

    // Should return 409 Conflict
    assert_eq!(resp.status().as_u16(), 409);

    // Parse body as JSON
    let body = test::read_body(resp).await;
    let problem: serde_json::Value =
        serde_json::from_slice(&body).expect("valid JSON problem details");

    // Verify standard Problem+JSON fields
    assert_eq!(problem["status"], 409);
    assert_eq!(problem["code"], "OPTIMISTIC_LOCK");
    assert!(problem["detail"]
        .as_str()
        .unwrap()
        .contains("expected version 5"));
    assert!(problem["detail"]
        .as_str()
        .unwrap()
        .contains("actual version 7"));
    assert!(problem["trace_id"].is_string());

    // Verify extensions field with version info
    let extensions = problem["extensions"]
        .as_object()
        .expect("extensions present");
    assert_eq!(extensions["expected"], 5);
    assert_eq!(extensions["actual"], 7);
}
