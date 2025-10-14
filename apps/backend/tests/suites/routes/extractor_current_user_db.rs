use actix_web::{test, HttpMessage};
use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::infra::state::build_state;
use backend::state::security_config::SecurityConfig;
use backend::utils::unique::{unique_email, unique_str};
use serde_json::Value;

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;
use crate::support::auth::mint_test_token;
use crate::support::factory::seed_user_with_sub;

#[actix_web::test]
async fn test_me_db_success() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and custom security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config.clone())
        .build()
        .await?;

    // Open SharedTxn for this test
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Seed user with specific sub - use unique helpers to ensure uniqueness
    let test_sub = unique_str("test-sub");
    let test_email = unique_email("test");
    let user = seed_user_with_sub(shared.transaction(), &test_sub, Some(&test_email))
        .await
        .expect("should create user successfully");

    // Mint JWT with the same sub
    let token = mint_test_token(&test_sub, &test_email, &security_config);

    // Build app with production routes
    let app = create_test_app(state).with_prod_routes().build().await?;

    // Make request with valid token
    let req = test::TestRequest::get()
        .uri("/api/private/me_db")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    // Inject SharedTxn via extensions
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;

    // Assert 200
    assert_eq!(resp.status().as_u16(), 200);

    // Read and parse response body
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    let response: Value = serde_json::from_str(&body_str).expect("should parse JSON");

    // Verify response structure
    assert_eq!(response["id"], user.id);
    assert_eq!(response["sub"], test_sub);
    assert_eq!(response["email"], Value::Null); // email is None since it's not in users table

    // Rollback the transaction
    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_me_db_user_not_found() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and custom security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config.clone())
        .build()
        .await?;

    // Mint JWT with a sub that doesn't exist in database - use unique helpers to ensure uniqueness
    let missing_sub = unique_str("missing-sub");
    let test_email = unique_email("missing");
    let token = mint_test_token(&missing_sub, &test_email, &security_config);

    // Build app with production routes
    let app = create_test_app(state).with_prod_routes().build().await?;

    // Make request with valid token but non-existent user
    let req = test::TestRequest::get()
        .uri("/api/private/me_db")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 403
    assert_eq!(resp.status().as_u16(), 403);

    // Validate error structure
    assert_problem_details_structure(
        resp,
        403,
        "FORBIDDEN_USER_NOT_FOUND",
        "User not found in database",
    )
    .await;

    Ok(())
}

#[actix_web::test]
async fn test_me_db_unauthorized() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and default security config
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Build app with production routes
    let app = create_test_app(state).with_prod_routes().build().await?;

    // Make request without Authorization header
    let req = test::TestRequest::get()
        .uri("/api/private/me_db")
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assert 401
    assert_eq!(resp.status().as_u16(), 401);

    // Validate error structure
    assert_problem_details_structure(
        resp,
        401,
        "UNAUTHORIZED_MISSING_BEARER",
        "Missing or malformed Bearer token",
    )
    .await;

    Ok(())
}
