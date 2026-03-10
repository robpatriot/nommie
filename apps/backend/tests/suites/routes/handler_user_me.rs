//! Integration tests for GET /api/user/me

use actix_http::Request;
use actix_web::body::BoxBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, HttpMessage};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::state::security_config::SecurityConfig;
use backend_test_support::unique_helpers::{unique_email, unique_str};
use serde_json::Value;

use crate::support::app_builder::create_test_app;
use crate::support::auth::mint_test_token;
use crate::support::factory::seed_user_with_sub;
use crate::support::test_state_builder;

async fn call_service_or_error<S>(app: &mut S, req: Request) -> ServiceResponse<BoxBody>
where
    S: actix_web::dev::Service<
        Request,
        Response = ServiceResponse<BoxBody>,
        Error = actix_web::Error,
    >,
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
async fn test_me_authenticated_returns_200_with_id_username_role(
) -> Result<(), Box<dyn std::error::Error>> {
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config.clone())
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;

    let test_sub = unique_str("me-test-sub");
    let test_email = unique_email("me-test");
    let user = seed_user_with_sub(shared.transaction(), &test_sub, Some(&test_email))
        .await
        .expect("should create user");

    let token = mint_test_token(&user.id.to_string(), &test_email, &security_config);

    let mut app = create_test_app(state).with_prod_routes().build().await?;

    let req = test::TestRequest::get()
        .uri("/api/user/me")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = call_service_or_error(&mut app, req).await;

    assert_eq!(resp.status().as_u16(), 200);

    let body = test::read_body(resp).await;
    let response: Value = serde_json::from_slice(&body)?;

    assert_eq!(response["id"], user.id);
    assert_eq!(response["username"].as_str(), user.username.as_deref());
    assert!(response["role"].as_str().is_some());
    let role = response["role"].as_str().unwrap();
    assert!(role == "user" || role == "admin");

    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn test_me_unauthenticated_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let state = test_state_builder()?.build().await?;
    let mut app = create_test_app(state).with_prod_routes().build().await?;

    let req = test::TestRequest::get().uri("/api/user/me").to_request();

    let resp = call_service_or_error(&mut app, req).await;

    assert_eq!(resp.status().as_u16(), 401);

    let body = test::read_body(resp).await;
    let detail = String::from_utf8(body.to_vec())?;
    assert_eq!(detail, "Missing Authorization header");

    Ok(())
}
