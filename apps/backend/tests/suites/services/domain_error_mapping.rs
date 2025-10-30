use actix_web::{test, web, App, HttpResponse};
use backend::errors::domain::{
    ConflictKind, DomainError, InfraErrorKind, NotFoundKind, ValidationKind,
};
use backend::AppError;

#[actix_web::test]
async fn problem_details_from_domain_errors() {
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
async fn adapter_unique_violation_maps_to_conflict() {
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
