use actix_http::Request;
use actix_web::body::BoxBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, HttpMessage};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::infra::state::build_state;
use backend::{AppError, CurrentUser};
use backend_test_support::unique_helpers::{unique_email, unique_str};
use serde_json::Value;

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;
use crate::support::factory::seed_user_with_sub;
use crate::support::test_middleware::TestSessionInjector;
use crate::support::test_state_builder;

#[derive(serde::Serialize)]
struct TestMeDbResponse {
    id: i64,
    email: Option<String>,
}

async fn test_me_db_handler(
    current_user: CurrentUser,
) -> Result<web::Json<TestMeDbResponse>, AppError> {
    Ok(web::Json(TestMeDbResponse {
        id: current_user.id,
        email: current_user.email,
    }))
}

async fn build_auth_app(
    state: backend::state::app_state::AppState,
    session_injector: Option<TestSessionInjector>,
) -> Result<
    impl actix_web::dev::Service<Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>,
    AppError,
> {
    let builder = create_test_app(state).with_routes(|cfg| {
        cfg.service(
            web::scope("/test-auth")
                .route("/me-db", web::get().to(test_me_db_handler)),
        );
    });

    if let Some(injector) = session_injector {
        builder.with_session(injector).build().await
    } else {
        builder.build().await
    }
}

async fn call_service_or_error<S>(app: &mut S, req: Request) -> ServiceResponse<BoxBody>
where
    S: Service<Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>,
{
    match app.call(req).await {
        Ok(resp) => resp,
        Err(err) => {
            let response = err.error_response().map_into_boxed_body();
            let dummy_request = test::TestRequest::default().to_http_request();
            ServiceResponse::new(dummy_request, response)
        }
    }
}

#[actix_web::test]
async fn test_me_db_success() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    // Open SharedTxn for this test
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(&db).await?;

    // Seed user
    let test_sub = unique_str("test-sub");
    let test_email = unique_email("test");
    let user = seed_user_with_sub(shared.transaction(), &test_sub, Some(&test_email))
        .await
        .expect("should create user successfully");

    let session_injector = TestSessionInjector::new(user.id, &test_sub, &test_email);

    // Build app with test routes
    let mut app = build_auth_app(state, Some(session_injector)).await?;

    // Make request
    let req = test::TestRequest::get()
        .uri("/test-auth/me-db")
        .to_request();

    // Inject SharedTxn via extensions
    req.extensions_mut().insert(shared.clone());

    let resp = call_service_or_error(&mut app, req).await;

    // Assert 200
    assert_eq!(resp.status().as_u16(), 200);

    // Read and parse response body
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    let response: Value = serde_json::from_str(&body_str).expect("should parse JSON");

    // Verify response structure
    assert_eq!(response["id"], user.id);
    assert_eq!(response["email"], Value::String(test_email.clone()));

    // Rollback the transaction
    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_me_db_user_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    // Inject a session with a user id that doesn't exist in the database
    let session_injector = TestSessionInjector::new(999999999_i64, "nonexistent-sub", "missing@example.com");

    // Build app with test routes
    let mut app = build_auth_app(state, Some(session_injector)).await?;

    // Make request with session pointing to non-existent user
    let req = test::TestRequest::get()
        .uri("/test-auth/me-db")
        .to_request();

    let resp = call_service_or_error(&mut app, req).await;

    // Assert 401 Unauthorized (user not found in database)
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure
    assert_problem_details_structure(resp, 401, "UNAUTHORIZED", "Authentication required").await;

    Ok(())
}

#[actix_web::test]
async fn test_me_db_unauthorized() -> Result<(), Box<dyn std::error::Error>> {
    // Build state without database (test doesn't need it)
    let state = build_state().build().await?;

    // Build app without session injector - no SessionData in extensions
    let mut app = build_auth_app(state, None).await?;

    // Make request without any session data
    let req = test::TestRequest::get()
        .uri("/test-auth/me-db")
        .to_request();

    let resp = call_service_or_error(&mut app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    Ok(())
}
