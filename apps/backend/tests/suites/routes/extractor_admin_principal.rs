//! AdminPrincipal extractor tests.

use actix_http::Request;
use actix_web::body::BoxBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, HttpMessage};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::users::UserRole;
use backend_test_support::unique_helpers::{unique_email, unique_str};
use serde_json::Value;

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;
use crate::support::factory::{seed_user_with_sub, seed_user_with_sub_and_role};
use crate::support::test_middleware::TestSessionInjector;
use crate::support::test_state_builder;

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
async fn admin_user_extractor_returns_principal() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;

    let test_sub = unique_str("admin-extractor-sub");
    let test_email = unique_email("admin-extractor");
    let user = seed_user_with_sub_and_role(
        shared.transaction(),
        &test_sub,
        Some(&test_email),
        UserRole::Admin,
    )
    .await
    .expect("should create admin user");

    let session_injector = TestSessionInjector::new(user.id, &test_sub, &test_email);
    let mut app = create_test_app(state)
        .with_prod_routes()
        .with_session(session_injector)
        .build()
        .await?;

    let req = test::TestRequest::get()
        .uri("/api/admin/users/search?q=test")
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = call_service_or_error(&mut app, req).await;

    assert_eq!(resp.status().as_u16(), 200);

    let body = test::read_body(resp).await;
    let response: Value = serde_json::from_slice(&body)?;
    assert!(response.get("items").is_some());
    assert!(response.get("next_cursor").is_some());

    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn non_admin_user_returns_403() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;

    let test_sub = unique_str("admin-extractor-user-sub");
    let test_email = unique_email("admin-extractor-user");
    let user = seed_user_with_sub(shared.transaction(), &test_sub, Some(&test_email))
        .await
        .expect("should create user");

    let session_injector = TestSessionInjector::new(user.id, &test_sub, &test_email);
    let mut app = create_test_app(state)
        .with_prod_routes()
        .with_session(session_injector)
        .build()
        .await?;

    let req = test::TestRequest::get()
        .uri("/api/admin/users/search?q=test")
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = call_service_or_error(&mut app, req).await;

    assert_eq!(resp.status().as_u16(), 403);
    assert_problem_details_structure(resp, 403, "PERMISSION_DENIED", "Admin access required").await;

    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn no_session_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;
    let mut app = create_test_app(state).with_prod_routes().build().await?;

    let req = test::TestRequest::get()
        .uri("/api/admin/users/search?q=test")
        .to_request();

    let resp = call_service_or_error(&mut app, req).await;

    assert_eq!(resp.status().as_u16(), 401);

    Ok(())
}
