//! Tests for AppError mappings and HTTP responses.
//!
//! This module tests the precise mapping of database errors to AppError variants
//! and ensures HTTP responses follow RFC 7807 Problem Details format.

use actix_web::{test, web, App, HttpResponse, Result};
use backend::error::AppError;
use backend::errors::ErrorCode;

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

        // Use the common helper for comprehensive validation
        use crate::common::assert_problem_details_structure;
        // For Unauthorized variants, we need to use the specific detail messages
        let expected_detail = match error {
            AppError::Unauthorized => "Authentication required",
            AppError::UnauthorizedMissingBearer => "Missing or malformed Bearer token",
            AppError::UnauthorizedInvalidJwt => "Invalid JWT",
            AppError::UnauthorizedExpiredJwt => "Token expired",
            _ => "Authentication required", // fallback
        };
        assert_problem_details_structure(resp, 401, error.code().as_str(), expected_detail).await;
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

    // Use the common helper for comprehensive validation
    use crate::common::assert_problem_details_structure;
    assert_problem_details_structure(resp, 503, "DB_UNAVAILABLE", "Database unavailable").await;
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

    // Use the common helper for comprehensive validation
    use crate::common::assert_problem_details_structure;
    assert_problem_details_structure(resp, 409, "UNIQUE_VIOLATION", "Duplicate key").await;

    // Test FkViolation
    let req = test::TestRequest::get().uri("/fk_violation").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);

    // Use the common helper for comprehensive validation
    assert_problem_details_structure(resp, 409, "FK_VIOLATION", "Foreign key constraint").await;
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

    // Use the common helper for comprehensive validation
    use crate::common::assert_problem_details_structure;
    assert_problem_details_structure(resp, 400, "CHECK_VIOLATION", "Check constraint failed").await;
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

    // Use the common helper for comprehensive validation
    use crate::common::assert_problem_details_structure;
    assert_problem_details_structure(resp, 404, "RECORD_NOT_FOUND", "Record not found").await;
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

    // Use the common helper for comprehensive validation
    use crate::common::assert_problem_details_structure;
    assert_problem_details_structure(resp, 500, "CONFIG_ERROR", "Missing environment variable")
        .await;
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

    // Use the common helper for comprehensive validation
    use crate::common::assert_problem_details_structure;
    assert_problem_details_structure(resp, 500, "DB_ERROR", "Database connection failed").await;
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
    let app_error: AppError = backend::infra::db_errors::map_db_err(db_err).into();

    assert!(matches!(app_error, AppError::NotFound { .. }));
    assert_eq!(app_error.code(), ErrorCode::NotFound);

    // Test ConnectionAcquire mapping (using a mock error since we can't easily construct ConnAcquireErr)
    // We'll test this via string matching in the main implementation

    // Test generic DbErr mapping -> Internal
    let db_err = DbErr::Custom("Some other error".to_string());
    let app_error: AppError = backend::infra::db_errors::map_db_err(db_err).into();

    assert!(matches!(app_error, AppError::Internal { .. }));
    assert_eq!(app_error.code(), ErrorCode::InternalError);
}

#[actix_web::test]
async fn test_sanitized_database_error_details() {
    use sea_orm::DbErr;

    // Test unique constraint violation with sanitized detail (via adapter)
    let db_err = DbErr::Custom(
        "duplicate key value violates unique constraint \"users_email_key\" SQLSTATE(23505)"
            .to_string(),
    );
    let app_error: AppError = backend::infra::db_errors::map_db_err(db_err).into();

    match app_error {
        AppError::Conflict { code, detail, .. } => {
            assert_eq!(code, ErrorCode::UniqueEmail);
            assert_eq!(detail, "Email already registered");
        }
        _ => panic!("Expected Conflict variant"),
    }

    // Test foreign key constraint violation with sanitized detail
    let db_err = DbErr::Custom(
        "insert or update on table \"games\" violates foreign key constraint SQLSTATE(23503)"
            .to_string(),
    );
    let app_error: AppError = backend::infra::db_errors::map_db_err(db_err).into();

    match app_error {
        AppError::Validation { code, status, .. } => {
            assert_eq!(code, ErrorCode::ValidationError);
            assert_eq!(status.as_u16(), 422);
        }
        _ => panic!("Expected Validation variant"),
    }

    // Test check constraint violation with sanitized detail
    let db_err = DbErr::Custom("new row for relation \"games\" violates check constraint \"games_status_check\" SQLSTATE(23514)".to_string());
    let app_error: AppError = backend::infra::db_errors::map_db_err(db_err).into();

    match app_error {
        AppError::Validation { code, status, .. } => {
            assert_eq!(code, ErrorCode::ValidationError);
            assert_eq!(status.as_u16(), 422);
        }
        _ => panic!("Expected Validation variant"),
    }

    // Test connection issue detection
    let db_err = DbErr::Custom("connection timeout after 30 seconds".to_string());
    let app_error: AppError = backend::infra::db_errors::map_db_err(db_err).into();

    assert_eq!(app_error.code(), ErrorCode::DbTimeout);

    // Test generic database error with sanitized detail
    let db_err = DbErr::Custom("some unexpected database error".to_string());
    let app_error: AppError = backend::infra::db_errors::map_db_err(db_err).into();

    // Maps to generic internal/infra error
    match app_error {
        AppError::Internal { detail, .. } => {
            assert_eq!(detail, "Database operation failed");
        }
        _ => panic!("Expected Internal variant"),
    }
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
                    let app_error: AppError = backend::infra::db_errors::map_db_err(db_err).into();
                    test_handler(app_error)
                }),
            )
            .route(
                "/fk_violation",
                web::get().to(|| {
                    let db_err = DbErr::Custom(
                        "foreign key constraint violation SQLSTATE(23503)".to_string(),
                    );
                    let app_error: AppError = backend::infra::db_errors::map_db_err(db_err).into();
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

    // Use the common helper for comprehensive validation
    use crate::common::assert_problem_details_structure;
    assert_problem_details_structure(resp, 409, "CONFLICT", "Unique constraint violation").await;

    // Test FK violation response has sanitized detail
    let req = test::TestRequest::get().uri("/fk_violation").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 422);

    // Use the common helper for comprehensive validation
    assert_problem_details_structure(
        resp,
        422,
        "VALIDATION_ERROR",
        "Foreign key constraint violation",
    )
    .await;
}
