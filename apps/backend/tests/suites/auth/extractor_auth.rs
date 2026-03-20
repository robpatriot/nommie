//! SessionExtract middleware tests.
//!
//! Tests that require Redis (REDIS_URL env var) exercise the full SessionExtract path.
//! Tests that only test route behavior use TestSessionInjector instead.

use actix_http::Request;
use actix_web::body::BoxBody;
use actix_web::dev::ServiceResponse;
use actix_web::http::StatusCode;
use actix_web::{test, web, HttpMessage};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::middleware::session_extract::SessionExtract;
use backend::{AppError, CurrentUser};
use backend_test_support::unique_helpers::{unique_email, unique_str};
use serde::Serialize;
use serde_json::Value;

use crate::support::app_builder::create_test_app;
use crate::support::auth::create_test_session;
use crate::support::factory::create_test_user;
use crate::support::test_state_builder;

#[derive(Serialize)]
struct TestCurrentUserResponse {
    id: i64,
    email: Option<String>,
}

async fn test_current_user_handler(
    current_user: CurrentUser,
) -> Result<web::Json<TestCurrentUserResponse>, AppError> {
    Ok(web::Json(TestCurrentUserResponse {
        id: current_user.id,
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
                    .wrap(SessionExtract)
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

/// Test: no cookie and no ?token= → 401 Unauthorized
#[actix_web::test]
async fn test_missing_cookie() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;
    let mut app = build_auth_test_app(state).await?;

    let req = test::TestRequest::get().uri("/test-auth/me").to_request();

    let (status, detail) = call_and_capture_error(&mut app, req).await?;
    assert_eq!(status.as_u16(), 401);
    assert_eq!(detail, "Unauthorized");

    Ok(())
}

/// Test: valid format token not in Redis → 401 UNAUTHORIZED_INVALID_TOKEN
/// Requires REDIS_URL.
#[actix_web::test]
async fn test_invalid_session_token() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    // Only run if Redis is configured
    if state.session_redis().is_none() {
        return Ok(());
    }

    let mut app = build_auth_test_app(state).await?;

    // Generate a token that looks valid but is not in Redis
    let fake_token = backend::auth::session::generate_session_token();
    let req = test::TestRequest::get()
        .uri("/test-auth/me")
        .insert_header(("Cookie", format!("backend_session={fake_token}")))
        .to_request();

    let (status, detail) = call_and_capture_error(&mut app, req).await?;
    assert_eq!(status.as_u16(), 401);
    assert_eq!(detail, "UnauthorizedInvalidToken");

    Ok(())
}

/// Test: valid session cookie → 200 with correct user data
/// Requires REDIS_URL.
#[actix_web::test]
async fn test_valid_session() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    // Only run if Redis is configured
    if state.session_redis().is_none() {
        return Ok(());
    }

    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(&db).await?;

    let user_sub = unique_str("session-test-sub");
    let user_email = unique_email("session-test");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Session User")).await?;

    let token = create_test_session(&state, user_id, &user_sub, &user_email).await?;

    let app = build_auth_test_app(state).await?;

    let req = test::TestRequest::get()
        .uri("/test-auth/me")
        .insert_header(("Cookie", format!("backend_session={token}")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["id"], user_id);
    assert_eq!(body["email"], user_email);

    shared.rollback().await?;

    Ok(())
}

/// Test: valid ws_token via ?token= query parameter → 200
/// Requires REDIS_URL.
#[actix_web::test]
async fn test_valid_ws_token() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    // Only run if Redis is configured
    if state.session_redis().is_none() {
        return Ok(());
    }

    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(&db).await?;

    let user_sub = unique_str("ws-token-test-sub");
    let user_email = unique_email("ws-token-test");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("WS Token User")).await?;

    let ws_token =
        crate::support::auth::create_test_ws_token(&state, user_id, &user_sub, &user_email)
            .await?;

    let app = build_auth_test_app(state).await?;

    let req = test::TestRequest::get()
        .uri(&format!("/test-auth/me?token={ws_token}"))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["id"], user_id);
    assert_eq!(body["email"], user_email);

    shared.rollback().await?;

    Ok(())
}
