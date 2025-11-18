use actix_http::Request;
use actix_web::body::BoxBody;
use actix_web::dev::ServiceResponse;
use actix_web::http::StatusCode;
use actix_web::{test, web, HttpMessage};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::error::AppError;
use backend::extractors::current_user::CurrentUser;
use backend::middleware::jwt_extract::JwtExtract;
use backend::state::security_config::SecurityConfig;
use backend::utils::unique::{unique_email, unique_str};
use serde::Serialize;
use serde_json::Value;

use crate::support::app_builder::create_test_app;
use crate::support::auth::{mint_expired_token, mint_test_token};
use crate::support::factory::create_test_user;
use crate::support::test_state_builder;

#[derive(Serialize)]
struct TestCurrentUserResponse {
    sub: String,
    email: Option<String>,
}

async fn test_current_user_handler(
    current_user: CurrentUser,
) -> Result<web::Json<TestCurrentUserResponse>, AppError> {
    Ok(web::Json(TestCurrentUserResponse {
        sub: current_user.sub,
        email: current_user.email,
    }))
}

async fn build_auth_test_app(
    state: backend::state::app_state::AppState,
) -> Result<
    impl actix_web::dev::Service<Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>,
    AppError,
> {
    create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/test-auth")
                    .wrap(JwtExtract)
                    .route("/me", web::get().to(test_current_user_handler)),
            );
        })
        .build()
        .await
}

async fn call_and_capture_error<S>(
    app: &mut S,
    req: Request,
) -> Result<(StatusCode, String), actix_web::Error>
where
    S: actix_web::dev::Service<
        Request,
        Response = ServiceResponse<BoxBody>,
        Error = actix_web::Error,
    >,
{
    let err = app.call(req).await.expect_err("expected error response");
    let status = err.as_response_error().status_code();
    let detail = err.to_string();
    Ok((status, detail))
}

#[actix_web::test]
async fn test_missing_header() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;
    let mut app = build_auth_test_app(state).await?;

    let req = test::TestRequest::get().uri("/test-auth/me").to_request();

    let (status, detail) = call_and_capture_error(&mut app, req).await?;
    assert_eq!(status.as_u16(), 401);
    assert_eq!(detail, "Missing Authorization header");

    Ok(())
}

#[actix_web::test]
async fn test_malformed_scheme() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;
    let mut app = build_auth_test_app(state).await?;

    // Test malformed Authorization header
    let req = test::TestRequest::get()
        .uri("/test-auth/me")
        .insert_header(("Authorization", "Token abc"))
        .to_request();

    let (status, detail) = call_and_capture_error(&mut app, req).await?;
    assert_eq!(status.as_u16(), 401);
    assert_eq!(detail, "Missing or invalid Bearer token");

    Ok(())
}

#[actix_web::test]
async fn test_empty_token() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;
    let mut app = build_auth_test_app(state).await?;

    // Test empty token
    let req = test::TestRequest::get()
        .uri("/test-auth/me")
        .insert_header(("Authorization", "Bearer "))
        .to_request();

    let (status, detail) = call_and_capture_error(&mut app, req).await?;
    assert_eq!(status.as_u16(), 401);
    assert_eq!(detail, "Missing or invalid Bearer token");

    Ok(())
}

#[actix_web::test]
async fn test_invalid_token() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;
    let mut app = build_auth_test_app(state).await?;

    // Test with invalid token
    let req = test::TestRequest::get()
        .uri("/test-auth/me")
        .insert_header(("Authorization", "Bearer not-a-real-token"))
        .to_request();

    let (status, detail) = call_and_capture_error(&mut app, req).await?;
    assert_eq!(status.as_u16(), 401);
    assert_eq!(detail, "UnauthorizedInvalidJwt");

    Ok(())
}

#[actix_web::test]
async fn test_expired_token() -> Result<(), Box<dyn std::error::Error>> {
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config.clone())
        .build()
        .await?;

    // Create expired JWT token by using a time from the past
    let sub = unique_str("test-sub-expired");
    let email = unique_email("test");
    let expired_token = mint_expired_token(&sub, &email, &security_config);

    let db = require_db(&state)?;
    let shared = SharedTxn::open(db).await?;
    create_test_user(shared.transaction(), &sub, Some("Expired User")).await?;

    let mut app = build_auth_test_app(state).await?;

    // Test with expired token
    let req = test::TestRequest::get()
        .uri("/test-auth/me")
        .insert_header(("Authorization", format!("Bearer {expired_token}")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let (status, detail) = call_and_capture_error(&mut app, req).await?;
    assert_eq!(status.as_u16(), 401);
    assert_eq!(detail, "UnauthorizedExpiredJwt");

    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_happy_path() -> Result<(), Box<dyn std::error::Error>> {
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config.clone())
        .build()
        .await?;

    // Create a valid JWT token
    let sub = unique_str("test-sub-happy");
    let email = unique_email("test");
    let token = mint_test_token(&sub, &email, &security_config);

    let db = require_db(&state)?;
    let shared = SharedTxn::open(db).await?;
    create_test_user(shared.transaction(), &sub, Some("Happy User")).await?;

    let app = build_auth_test_app(state).await?;

    // Make request with valid token
    let req = test::TestRequest::get()
        .uri("/test-auth/me")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["sub"], sub);
    assert_eq!(body["email"], email);

    shared.rollback().await?;

    Ok(())
}
