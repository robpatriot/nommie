//! Admin users API handler tests.

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
async fn get_search_200_with_valid_params_admin_user() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;

    let test_sub = unique_str("handler-admin-search");
    let test_email = unique_email("handler-admin-search");
    let admin = seed_user_with_sub_and_role(
        shared.transaction(),
        &test_sub,
        Some(&test_email),
        UserRole::Admin,
    )
    .await
    .expect("should create admin");

    let session_injector = TestSessionInjector::new(admin.id, &test_sub, &test_email);
    let mut app = create_test_app(state)
        .with_prod_routes()
        .with_session(session_injector)
        .build()
        .await?;

    let req = test::TestRequest::get()
        .uri("/api/admin/users/search?q=user")
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
async fn get_search_403_for_non_admin() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;

    let test_sub = unique_str("handler-admin-nonadmin");
    let test_email = unique_email("handler-admin-nonadmin");
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
        .uri("/api/admin/users/search?q=user")
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = call_service_or_error(&mut app, req).await;

    assert_eq!(resp.status().as_u16(), 403);
    assert_problem_details_structure(resp, 403, "PERMISSION_DENIED", "Admin access required").await;

    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn post_grant_admin_200_body_has_user_and_changed() -> Result<(), Box<dyn std::error::Error>>
{
    let state = test_state_builder()?.build().await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;

    let admin_sub = unique_str("handler-grant-admin");
    let admin_email = unique_email("handler-grant-admin");
    let admin = seed_user_with_sub_and_role(
        shared.transaction(),
        &admin_sub,
        Some(&admin_email),
        UserRole::Admin,
    )
    .await
    .expect("should create admin");

    let target_sub = unique_str("handler-grant-target");
    let target_email = unique_email("handler-grant-target");
    let target = seed_user_with_sub(shared.transaction(), &target_sub, Some(&target_email))
        .await
        .expect("should create target user");

    let session_injector = TestSessionInjector::new(admin.id, &admin_sub, &admin_email);
    let mut app = create_test_app(state)
        .with_prod_routes()
        .with_session(session_injector)
        .build()
        .await?;

    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/grant-admin", target.id))
        .insert_header(("content-type", "application/json"))
        .set_payload(r#"{}"#)
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = call_service_or_error(&mut app, req).await;

    assert_eq!(resp.status().as_u16(), 200);

    let body = test::read_body(resp).await;
    let response: Value = serde_json::from_slice(&body)?;
    assert_eq!(response["user"]["id"], target.id);
    assert_eq!(response["user"]["role"], "admin");
    assert_eq!(response["changed"], true);

    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn post_grant_admin_404_for_non_existent_user() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;

    let admin_sub = unique_str("handler-grant-404");
    let admin_email = unique_email("handler-grant-404");
    let admin = seed_user_with_sub_and_role(
        shared.transaction(),
        &admin_sub,
        Some(&admin_email),
        UserRole::Admin,
    )
    .await
    .expect("should create admin");

    let session_injector = TestSessionInjector::new(admin.id, &admin_sub, &admin_email);
    let mut app = create_test_app(state)
        .with_prod_routes()
        .with_session(session_injector)
        .build()
        .await?;

    let non_existent_user_id = 999999999i64;

    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/admin/users/{}/grant-admin",
            non_existent_user_id
        ))
        .insert_header(("content-type", "application/json"))
        .set_payload(r#"{}"#)
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = call_service_or_error(&mut app, req).await;

    assert_eq!(resp.status().as_u16(), 404);
    assert_problem_details_structure(resp, 404, "TARGET_USER_NOT_FOUND", "Target user not found")
        .await;

    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn post_revoke_admin_409_for_self_revoke() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;

    let admin_sub = unique_str("handler-revoke-self");
    let admin_email = unique_email("handler-revoke-self");
    let admin = seed_user_with_sub_and_role(
        shared.transaction(),
        &admin_sub,
        Some(&admin_email),
        UserRole::Admin,
    )
    .await
    .expect("should create admin");

    let session_injector = TestSessionInjector::new(admin.id, &admin_sub, &admin_email);
    let mut app = create_test_app(state)
        .with_prod_routes()
        .with_session(session_injector)
        .build()
        .await?;

    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/revoke-admin", admin.id))
        .insert_header(("content-type", "application/json"))
        .set_payload(r#"{}"#)
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = call_service_or_error(&mut app, req).await;

    assert_eq!(resp.status().as_u16(), 409);
    assert_problem_details_structure(
        resp,
        409,
        "CANNOT_REVOKE_OWN_ADMIN",
        "Cannot revoke admin from yourself",
    )
    .await;

    shared.rollback().await?;
    Ok(())
}

// Note: LAST_ADMIN_PROTECTION is tested at the service layer. Via the API it's unreachable
// because the actor must be admin (AdminPrincipal), so we always have at least one admin.
