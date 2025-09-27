//! Tests for AppError mappings and HTTP responses.
//!
//! This module tests the precise mapping of database errors to AppError variants
//! and ensures HTTP responses follow RFC 7807 Problem Details format.

use actix_web::{test, web, App, HttpResponse, Result};
use backend::error::AppError;
use backend::errors::ErrorCode;
use serde_json::Value;

/// Test handler that returns a specific AppError for testing
async fn test_handler(error: AppError) -> Result<HttpResponse, AppError> {
    Err(error)
}

#[actix_web::test]
async fn test_unauthorized_responses() {
    let app = test::init_service(
        App::new()
            .route(
                "/unauthorized",
                web::get().to(|| test_handler(AppError::unauthorized())),
            )
            .route(
                "/unauthorized_missing_bearer",
                web::get().to(|| test_handler(AppError::unauthorized_missing_bearer())),
            )
            .route(
                "/unauthorized_invalid_jwt",
                web::get().to(|| test_handler(AppError::unauthorized_invalid_jwt())),
            )
            .route(
                "/unauthorized_expired_jwt",
                web::get().to(|| test_handler(AppError::unauthorized_expired_jwt())),
            ),
    )
    .await;

    // Test each Unauthorized variant
    let unauthorized_variants = vec![
        ("/unauthorized", AppError::unauthorized()),
        (
            "/unauthorized_missing_bearer",
            AppError::unauthorized_missing_bearer(),
        ),
        (
            "/unauthorized_invalid_jwt",
            AppError::unauthorized_invalid_jwt(),
        ),
        (
            "/unauthorized_expired_jwt",
            AppError::unauthorized_expired_jwt(),
        ),
    ];

    for (path, error) in unauthorized_variants {
        let req = test::TestRequest::get().uri(path).to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 401);

        // Check WWW-Authenticate header
        let www_auth = resp.headers().get("WWW-Authenticate");
        assert!(www_auth.is_some());
        assert_eq!(www_auth.unwrap(), "Bearer");

        // Check x-trace-id header
        let trace_id = resp.headers().get("x-trace-id");
        assert!(trace_id.is_some());

        // Check content type
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/problem+json"
        );

        // Parse and validate JSON response
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], 401);
        assert_eq!(body["code"], error.code().as_str());
        assert!(!body["trace_id"].as_str().unwrap().is_empty());
        assert!(body["detail"].is_string());
        assert!(body["title"].is_string());
        assert!(body["type"].is_string());
    }
}

#[actix_web::test]
async fn test_database_unavailable_response() {
    let app = test::init_service(App::new().route(
        "/db_unavailable",
        web::get().to(|| test_handler(AppError::db_unavailable())),
    ))
    .await;

    let req = test::TestRequest::get().uri("/db_unavailable").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 503);

    // Check Retry-After header
    let retry_after = resp.headers().get("Retry-After");
    assert!(retry_after.is_some());
    assert_eq!(retry_after.unwrap(), "1");

    // Check x-trace-id header
    let trace_id = resp.headers().get("x-trace-id");
    assert!(trace_id.is_some());

    // Check content type
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/problem+json"
    );

    // Parse and validate JSON response
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], 503);
    assert_eq!(body["code"], "DB_UNAVAILABLE");
    assert!(!body["trace_id"].as_str().unwrap().is_empty());
    assert_eq!(body["detail"], "Database unavailable");
}

#[actix_web::test]
async fn test_conflict_responses() {
    let app = test::init_service(
        App::new()
            .route(
                "/unique_violation",
                web::get().to(|| {
                    test_handler(AppError::conflict(
                        ErrorCode::UniqueViolation,
                        "Duplicate key",
                    ))
                }),
            )
            .route(
                "/fk_violation",
                web::get().to(|| {
                    test_handler(AppError::conflict(
                        ErrorCode::FkViolation,
                        "Foreign key constraint",
                    ))
                }),
            ),
    )
    .await;

    // Test UniqueViolation
    let req = test::TestRequest::get()
        .uri("/unique_violation")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], 409);
    assert_eq!(body["code"], "UNIQUE_VIOLATION");
    assert_eq!(body["detail"], "Duplicate key");

    // Test FkViolation
    let req = test::TestRequest::get().uri("/fk_violation").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], 409);
    assert_eq!(body["code"], "FK_VIOLATION");
    assert_eq!(body["detail"], "Foreign key constraint");
}

#[actix_web::test]
async fn test_bad_request_response() {
    let app = test::init_service(App::new().route(
        "/check_violation",
        web::get().to(|| {
            test_handler(AppError::bad_request(
                ErrorCode::CheckViolation,
                "Check constraint failed",
            ))
        }),
    ))
    .await;

    let req = test::TestRequest::get()
        .uri("/check_violation")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], 400);
    assert_eq!(body["code"], "CHECK_VIOLATION");
    assert_eq!(body["detail"], "Check constraint failed");
}

#[actix_web::test]
async fn test_not_found_response() {
    let app = test::init_service(App::new().route(
        "/record_not_found",
        web::get().to(|| {
            test_handler(AppError::not_found(
                ErrorCode::RecordNotFound,
                "Record not found",
            ))
        }),
    ))
    .await;

    let req = test::TestRequest::get()
        .uri("/record_not_found")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], 404);
    assert_eq!(body["code"], "RECORD_NOT_FOUND");
    assert_eq!(body["detail"], "Record not found");
}

#[actix_web::test]
async fn test_config_error_response() {
    let app = test::init_service(App::new().route(
        "/config_error",
        web::get().to(|| test_handler(AppError::config("Missing environment variable"))),
    ))
    .await;

    let req = test::TestRequest::get().uri("/config_error").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 500);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], 500);
    assert_eq!(body["code"], "CONFIG_ERROR");
    assert_eq!(body["detail"], "Missing environment variable");
}

#[actix_web::test]
async fn test_db_error_response() {
    let app = test::init_service(App::new().route(
        "/db_error",
        web::get().to(|| test_handler(AppError::db("Database connection failed"))),
    ))
    .await;

    let req = test::TestRequest::get().uri("/db_error").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 500);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], 500);
    assert_eq!(body["code"], "DB_ERROR");
    assert_eq!(body["detail"], "Database connection failed");
}

#[actix_web::test]
async fn test_environment_variable_error_mapping() {
    use std::env::VarError;

    // Test that VarError maps to Config error
    let var_error = VarError::NotPresent;
    let app_error: AppError = var_error.into();

    assert!(matches!(app_error, AppError::Config { .. }));
    assert_eq!(app_error.code(), ErrorCode::ConfigError);
}

#[actix_web::test]
async fn test_sea_orm_error_mapping() {
    use sea_orm::DbErr;

    // Test RecordNotFound mapping
    let db_err = DbErr::RecordNotFound("Record not found".to_string());
    let app_error: AppError = db_err.into();

    assert!(matches!(app_error, AppError::NotFound { .. }));
    assert_eq!(app_error.code(), ErrorCode::RecordNotFound);

    // Test ConnectionAcquire mapping (using a mock error since we can't easily construct ConnAcquireErr)
    // We'll test this via string matching in the main implementation

    // Test generic DbErr mapping
    let db_err = DbErr::Custom("Some other error".to_string());
    let app_error: AppError = db_err.into();

    assert!(matches!(app_error, AppError::Db { .. }));
    assert_eq!(app_error.code(), ErrorCode::DbError);
}

#[actix_web::test]
async fn test_sanitized_database_error_details() {
    use sea_orm::DbErr;

    // Test unique constraint violation with sanitized detail
    let db_err = DbErr::Custom(
        "duplicate key value violates unique constraint \"users_email_key\" SQLSTATE(23505)"
            .to_string(),
    );
    let app_error: AppError = db_err.into();

    assert!(matches!(app_error, AppError::Conflict { .. }));
    assert_eq!(app_error.code(), ErrorCode::UniqueViolation);
    assert_eq!(app_error.detail(), "Unique constraint violation");

    // Test foreign key constraint violation with sanitized detail
    let db_err = DbErr::Custom(
        "insert or update on table \"games\" violates foreign key constraint SQLSTATE(23503)"
            .to_string(),
    );
    let app_error: AppError = db_err.into();

    assert!(matches!(app_error, AppError::Conflict { .. }));
    assert_eq!(app_error.code(), ErrorCode::FkViolation);
    assert_eq!(app_error.detail(), "Foreign key constraint violation");

    // Test check constraint violation with sanitized detail
    let db_err = DbErr::Custom("new row for relation \"games\" violates check constraint \"games_status_check\" SQLSTATE(23514)".to_string());
    let app_error: AppError = db_err.into();

    assert!(matches!(app_error, AppError::BadRequest { .. }));
    assert_eq!(app_error.code(), ErrorCode::CheckViolation);
    assert_eq!(app_error.detail(), "Check constraint violation");

    // Test connection issue detection
    let db_err = DbErr::Custom("connection timeout after 30 seconds".to_string());
    let app_error: AppError = db_err.into();

    assert!(matches!(app_error, AppError::DbUnavailable));
    assert_eq!(app_error.code(), ErrorCode::DbUnavailable);

    // Test generic database error with sanitized detail
    let db_err = DbErr::Custom("some unexpected database error".to_string());
    let app_error: AppError = db_err.into();

    assert!(matches!(app_error, AppError::Db { .. }));
    assert_eq!(app_error.code(), ErrorCode::DbError);
    assert_eq!(app_error.detail(), "Database operation failed");
}

#[actix_web::test]
async fn test_sanitized_error_http_responses() {
    use sea_orm::DbErr;

    let app = test::init_service(
        App::new()
            .route(
                "/unique_violation",
                web::get().to(|| {
                    let db_err = DbErr::Custom(
                        "duplicate key value violates unique constraint SQLSTATE(23505)"
                            .to_string(),
                    );
                    let app_error: AppError = db_err.into();
                    test_handler(app_error)
                }),
            )
            .route(
                "/fk_violation",
                web::get().to(|| {
                    let db_err = DbErr::Custom(
                        "foreign key constraint violation SQLSTATE(23503)".to_string(),
                    );
                    let app_error: AppError = db_err.into();
                    test_handler(app_error)
                }),
            ),
    )
    .await;

    // Test unique violation response has sanitized detail
    let req = test::TestRequest::get()
        .uri("/unique_violation")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], 409);
    assert_eq!(body["code"], "UNIQUE_VIOLATION");
    assert_eq!(body["detail"], "Unique constraint violation");
    assert!(!body["trace_id"].as_str().unwrap().is_empty());

    // Test FK violation response has sanitized detail
    let req = test::TestRequest::get().uri("/fk_violation").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], 409);
    assert_eq!(body["code"], "FK_VIOLATION");
    assert_eq!(body["detail"], "Foreign key constraint violation");
    assert!(!body["trace_id"].as_str().unwrap().is_empty());
}
