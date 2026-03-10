//! Check-allowlist endpoint tests.
//!
//! Verifies behavior when sub is provided and when it is absent:
//! - Returning user with same sub but changed email is allowed
//! - First-time user follows admission rules
//! - Missing sub falls back to email-based check

use actix_web::{test, HttpMessage};
use backend::db::require_db;
use backend::entities::allowed_emails;
use backend::prelude::SharedTxn;
use backend::state::admission_mode::AdmissionMode;
use backend::state::security_config::SecurityConfig;
use backend_test_support::unique_helpers::{unique_email, unique_str};
use sea_orm::EntityTrait;
use serde_json::json;

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;
use crate::support::auth::seed_admission_email;
use crate::support::test_state_builder;

const PROVIDER_GOOGLE: &str = "google";

#[actix_web::test]
async fn test_returning_user_with_sub_and_changed_email_allowed(
) -> Result<(), Box<dyn std::error::Error>> {
    let original_email = unique_email("original");
    let new_email = unique_email("new");
    let google_sub = unique_str("google");

    let state = test_state_builder()?
        .with_security(SecurityConfig::new(
            "test_secret_key_for_testing_purposes_only".as_bytes(),
        ))
        .with_admission_mode(AdmissionMode::Restricted)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;
    let txn = shared.transaction();

    let user = backend::repos::users::create_user(
        txn,
        "user",
        false,
        backend::entities::users::UserRole::User,
    )
    .await
    .map_err(backend::AppError::from)?;
    backend::repos::auth_identities::create_identity(
        txn,
        user.id,
        PROVIDER_GOOGLE,
        &google_sub,
        &original_email.to_lowercase(),
    )
    .await
    .map_err(backend::AppError::from)?;

    let app = create_test_app(state).with_prod_routes().build().await?;

    let body = json!({
        "email": new_email,
        "sub": google_sub
    });
    let req = test::TestRequest::post()
        .uri("/api/auth/check-allowlist")
        .set_json(body)
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "returning user with same sub but changed email should be allowed"
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.get("allowed").and_then(|v| v.as_bool()), Some(true));

    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn test_first_time_user_with_sub_follows_admission_rules(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("first-time");
    let google_sub = unique_str("google");

    let state = test_state_builder()?
        .with_security(SecurityConfig::new(
            "test_secret_key_for_testing_purposes_only".as_bytes(),
        ))
        .with_admission_mode(AdmissionMode::Restricted)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    allowed_emails::Entity::delete_many().exec(&db).await?;
    seed_admission_email(&db, "other@example.com", false).await;

    let shared = SharedTxn::open(&db).await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let body = json!({
        "email": test_email,
        "sub": google_sub
    });
    let req = test::TestRequest::post()
        .uri("/api/auth/check-allowlist")
        .set_json(body)
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert_problem_details_structure(
        resp,
        403,
        "EMAIL_NOT_ALLOWED",
        "Access restricted. Please contact support if you believe this is an error.",
    )
    .await;

    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn test_first_time_user_with_sub_admitted_allowed() -> Result<(), Box<dyn std::error::Error>>
{
    let test_email = unique_email("admitted");
    let google_sub = unique_str("google");

    let state = test_state_builder()?
        .with_security(SecurityConfig::new(
            "test_secret_key_for_testing_purposes_only".as_bytes(),
        ))
        .with_admission_mode(AdmissionMode::Restricted)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    seed_admission_email(&db, &test_email.to_lowercase(), false).await;

    let shared = SharedTxn::open(&db).await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let body = json!({
        "email": test_email,
        "sub": google_sub
    });
    let req = test::TestRequest::post()
        .uri("/api/auth/check-allowlist")
        .set_json(body)
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "first-time admitted user with sub should be allowed"
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.get("allowed").and_then(|v| v.as_bool()), Some(true));

    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn test_missing_sub_falls_back_to_email_check() -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("existing");
    let google_sub = unique_str("google");

    let state = test_state_builder()?
        .with_security(SecurityConfig::new(
            "test_secret_key_for_testing_purposes_only".as_bytes(),
        ))
        .with_admission_mode(AdmissionMode::Restricted)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;
    let txn = shared.transaction();

    let user = backend::repos::users::create_user(
        txn,
        "user",
        false,
        backend::entities::users::UserRole::User,
    )
    .await
    .map_err(backend::AppError::from)?;
    backend::repos::auth_identities::create_identity(
        txn,
        user.id,
        PROVIDER_GOOGLE,
        &google_sub,
        &test_email.to_lowercase(),
    )
    .await
    .map_err(backend::AppError::from)?;

    let app = create_test_app(state).with_prod_routes().build().await?;

    let body = json!({ "email": test_email });
    let req = test::TestRequest::post()
        .uri("/api/auth/check-allowlist")
        .set_json(body)
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "returning user without sub but with matching email should be allowed"
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.get("allowed").and_then(|v| v.as_bool()), Some(true));

    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn test_empty_sub_falls_back_to_email_check() -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("existing");
    let google_sub = unique_str("google");

    let state = test_state_builder()?
        .with_security(SecurityConfig::new(
            "test_secret_key_for_testing_purposes_only".as_bytes(),
        ))
        .with_admission_mode(AdmissionMode::Restricted)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;
    let txn = shared.transaction();

    let user = backend::repos::users::create_user(
        txn,
        "user",
        false,
        backend::entities::users::UserRole::User,
    )
    .await
    .map_err(backend::AppError::from)?;
    backend::repos::auth_identities::create_identity(
        txn,
        user.id,
        PROVIDER_GOOGLE,
        &google_sub,
        &test_email.to_lowercase(),
    )
    .await
    .map_err(backend::AppError::from)?;

    let app = create_test_app(state).with_prod_routes().build().await?;

    let body = json!({ "email": test_email, "sub": "" });
    let req = test::TestRequest::post()
        .uri("/api/auth/check-allowlist")
        .set_json(body)
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "empty sub should fall back to email check"
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.get("allowed").and_then(|v| v.as_bool()), Some(true));

    shared.rollback().await?;
    Ok(())
}
