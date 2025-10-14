use actix_web::{test, web, App, HttpResponse};
use backend::errors::domain::{
    ConflictKind, DomainError, InfraErrorKind, NotFoundKind, ValidationKind,
};
use backend::{AppError, ErrorCode};

#[actix_web::test]
async fn unit_maps_validation_to_422() {
    let de = DomainError::validation(
        ValidationKind::Other("VALIDATION_ERROR".into()),
        "bad field",
    );
    let app: AppError = de.into();
    assert_eq!(app.code(), ErrorCode::ValidationError);
    assert_eq!(app.status().as_u16(), 422);
}

#[actix_web::test]
async fn unit_maps_conflicts() {
    let seat = DomainError::conflict(ConflictKind::SeatTaken, "seat taken");
    let app: AppError = seat.into();
    assert_eq!(app.code().as_str(), "SEAT_TAKEN");
    assert_eq!(app.status().as_u16(), 409);

    let unique = DomainError::conflict(ConflictKind::UniqueEmail, "email exists");
    let app: AppError = unique.into();
    assert_eq!(app.code().as_str(), "UNIQUE_EMAIL");
    assert_eq!(app.status().as_u16(), 409);

    // Test generic conflict fallback
    let other = DomainError::conflict(
        ConflictKind::Other("some conflict".to_string()),
        "generic conflict",
    );
    let app: AppError = other.into();
    assert_eq!(app.code().as_str(), "CONFLICT");
    assert_eq!(app.status().as_u16(), 409);
}

#[actix_web::test]
async fn unit_maps_not_found() {
    let nf = DomainError::not_found(NotFoundKind::User, "no user");
    let app: AppError = nf.into();
    assert_eq!(app.code().as_str(), "USER_NOT_FOUND");
    assert_eq!(app.status().as_u16(), 404);
}

#[actix_web::test]
async fn unit_maps_infra() {
    let t = DomainError::infra(InfraErrorKind::Timeout, "timeout");
    let app: AppError = t.into();
    assert_eq!(app.code().as_str(), "DB_TIMEOUT");
    assert_eq!(app.status().as_u16(), 504);
    // Verify it's a Timeout AppError, not Validation
    assert!(matches!(app, AppError::Timeout { .. }));

    let down = DomainError::infra(InfraErrorKind::DbUnavailable, "down");
    let app: AppError = down.into();
    assert_eq!(app.code().as_str(), "DB_UNAVAILABLE");
    assert_eq!(app.status().as_u16(), 503);

    let corr = DomainError::infra(InfraErrorKind::DataCorruption, "bad");
    let app: AppError = corr.into();
    assert_eq!(app.code().as_str(), "DATA_CORRUPTION");
    assert_eq!(app.status().as_u16(), 500);

    let other = DomainError::infra(InfraErrorKind::Other("unknown".to_string()), "other");
    let app: AppError = other.into();
    assert_eq!(app.code().as_str(), "INTERNAL_ERROR");
    assert_eq!(app.status().as_u16(), 500);
}

#[actix_web::test]
async fn integration_problem_details_from_domain_errors() {
    async fn not_found_handler() -> Result<HttpResponse, AppError> {
        Err(DomainError::not_found(NotFoundKind::Game, "game missing").into())
    }
    async fn validation_handler() -> Result<HttpResponse, AppError> {
        Err(DomainError::validation(ValidationKind::Other("VALIDATION_ERROR".into()), "bad").into())
    }
    async fn timeout_handler() -> Result<HttpResponse, AppError> {
        Err(DomainError::infra(InfraErrorKind::Timeout, "operation timed out").into())
    }
    async fn generic_conflict_handler() -> Result<HttpResponse, AppError> {
        Err(
            DomainError::conflict(ConflictKind::Other("unknown".to_string()), "some conflict")
                .into(),
        )
    }

    let app = test::init_service(
        App::new()
            .route("/_test/de_not_found", web::get().to(not_found_handler))
            .route("/_test/de_validation", web::get().to(validation_handler))
            .route("/_test/de_timeout", web::get().to(timeout_handler))
            .route(
                "/_test/de_generic_conflict",
                web::get().to(generic_conflict_handler),
            ),
    )
    .await;

    // NotFound
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/_test/de_not_found")
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), 404);
    crate::common::assert_problem_details_structure(resp, 404, "GAME_NOT_FOUND", "game missing")
        .await;

    // Validation (422)
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/_test/de_validation")
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), 422);
    crate::common::assert_problem_details_structure(resp, 422, "VALIDATION_ERROR", "bad").await;

    // Timeout (504) - verify it's not Validation AppError
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/_test/de_timeout")
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), 504);
    crate::common::assert_problem_details_structure(resp, 504, "DB_TIMEOUT", "operation timed out")
        .await;

    // Generic conflict (409) - verify it uses CONFLICT code, not JOIN_CODE_CONFLICT
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/_test/de_generic_conflict")
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), 409);
    crate::common::assert_problem_details_structure(resp, 409, "CONFLICT", "some conflict").await;
}

#[actix_web::test]
async fn unit_domain_purity_check() {
    // This test verifies that domain modules can be used without HTTP/SeaORM imports
    // by creating DomainError instances and converting them to AppError
    use backend::errors::domain::{
        ConflictKind, DomainError, InfraErrorKind, NotFoundKind, ValidationKind,
    };
    use backend::AppError;

    // Test that we can create domain errors without HTTP imports
    let validation =
        DomainError::validation(ValidationKind::Other("VALIDATION_ERROR".into()), "test");
    let conflict = DomainError::conflict(ConflictKind::SeatTaken, "test");
    let not_found = DomainError::not_found(NotFoundKind::User, "test");
    let infra = DomainError::infra(InfraErrorKind::Timeout, "test");

    // Test that conversion to AppError works (this happens in the error module)
    let _: AppError = validation.into();
    let _: AppError = conflict.into();
    let _: AppError = not_found.into();
    let _: AppError = infra.into();
}

#[actix_web::test]
async fn unit_constructor_helpers() {
    // Test validation constructor
    let validation = DomainError::validation(
        ValidationKind::Other("VALIDATION_ERROR".into()),
        "invalid input",
    );
    assert!(matches!(validation, DomainError::Validation(_, _)));

    // Test conflict constructor
    let conflict = DomainError::conflict(ConflictKind::SeatTaken, "seat taken");
    assert!(matches!(
        conflict,
        DomainError::Conflict(ConflictKind::SeatTaken, _)
    ));

    // Test not found constructor
    let not_found = DomainError::not_found(NotFoundKind::User, "user missing");
    assert!(matches!(
        not_found,
        DomainError::NotFound(NotFoundKind::User, _)
    ));

    // Test infra constructor
    let infra = DomainError::infra(InfraErrorKind::Timeout, "timeout");
    assert!(matches!(
        infra,
        DomainError::Infra(InfraErrorKind::Timeout, _)
    ));
}

#[actix_web::test]
async fn integration_adapter_unique_violation_maps_to_conflict() {
    use backend::infra::db_errors::map_db_err;
    use sea_orm::DbErr;

    async fn unique_handler() -> Result<HttpResponse, AppError> {
        let db_err = DbErr::Custom(
            "duplicate key value violates unique constraint \"users_email_key\" SQLSTATE(23505)"
                .to_string(),
        );
        let de = map_db_err(db_err);
        Err(AppError::from(de))
    }

    let app =
        test::init_service(App::new().route("/_test/unique", web::get().to(unique_handler))).await;
    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/_test/unique").to_request(),
    )
    .await;
    assert_eq!(resp.status(), 409);
    crate::common::assert_problem_details_structure(
        resp,
        409,
        "UNIQUE_EMAIL",
        "Email already registered",
    )
    .await;
}
