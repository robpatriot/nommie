//! Auth admission flow tests.
//!
//! Tests for database-backed admission table, bootstrap seeding, and first-time login.

use std::env;
use std::sync::Arc;

use actix_web::{test, HttpMessage};
use backend::auth::google::{MockGoogleVerifier, VerifiedGoogleClaims};
use backend::db::require_db;
use backend::db::txn::with_txn;
use backend::entities::allowed_emails;
use backend::prelude::SharedTxn;
use backend::state::security_config::SecurityConfig;
use backend_test_support::unique_helpers::{unique_email, unique_str};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::json;

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;
use crate::support::test_state_builder;

const PROVIDER_GOOGLE: &str = "google";

/// Insert an email into the admission table for testing. Idempotent - uses ON CONFLICT DO NOTHING.
async fn seed_admission_email(conn: &sea_orm::DatabaseConnection, email: &str) {
    use sea_orm::sea_query::OnConflict;

    let now = time::OffsetDateTime::now_utc();
    let model = allowed_emails::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        email: sea_orm::ActiveValue::Set(email.to_string()),
        created_at: sea_orm::ActiveValue::Set(now),
    };
    let _ = allowed_emails::Entity::insert(model)
        .on_conflict(
            OnConflict::columns([allowed_emails::Column::Email])
                .do_nothing()
                .to_owned(),
        )
        .exec(conn)
        .await;
}

#[actix_web::test]
async fn test_admitted_first_time_login_creates_user_and_identity(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("admitted");
    let test_google_sub = unique_str("google");
    let mock_verifier = Arc::new(MockGoogleVerifier::new(VerifiedGoogleClaims {
        sub: test_google_sub.clone(),
        email: test_email.clone(),
        name: Some("Test User".to_string()),
    }));

    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = test_state_builder()?
        .with_security(security_config.clone())
        .with_google_verifier(mock_verifier)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    seed_admission_email(&db, &test_email.to_lowercase()).await;

    let shared = SharedTxn::open(&db).await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let login_data = json!({ "id_token": "test-token" });
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "admitted first-time login should succeed"
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("token").is_some());

    let identity_count = backend::entities::user_auth_identities::Entity::find()
        .filter(backend::entities::user_auth_identities::Column::Provider.eq(PROVIDER_GOOGLE))
        .filter(
            backend::entities::user_auth_identities::Column::Email.eq(test_email.to_lowercase()),
        )
        .count(shared.transaction())
        .await?;
    assert_eq!(identity_count, 1, "should have exactly one identity");

    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn test_non_admitted_first_time_login_denied() -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("not-admitted");
    let test_google_sub = unique_str("google");
    let mock_verifier = Arc::new(MockGoogleVerifier::new(VerifiedGoogleClaims {
        sub: test_google_sub,
        email: test_email.clone(),
        name: None,
    }));

    let state = test_state_builder()?
        .with_security(SecurityConfig::new(
            "test_secret_key_for_testing_purposes_only".as_bytes(),
        ))
        .with_google_verifier(mock_verifier)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    let other_email = backend_test_support::unique_helpers::unique_email("other");
    // Clear table so bootstrap ALLOWED_EMAILS cannot admit test_email; insert only other_email
    allowed_emails::Entity::delete_many().exec(&db).await?;
    seed_admission_email(&db, &other_email).await;

    let shared = SharedTxn::open(&db).await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let login_data = json!({ "id_token": "test-token" });
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
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
async fn test_existing_linked_user_logs_in_without_admission_check(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("existing");
    let test_google_sub = unique_str("google");
    let mock_verifier = Arc::new(MockGoogleVerifier::new(VerifiedGoogleClaims {
        sub: test_google_sub.clone(),
        email: test_email.clone(),
        name: None,
    }));

    let state = test_state_builder()?
        .with_security(SecurityConfig::new(
            "test_secret_key_for_testing_purposes_only".as_bytes(),
        ))
        .with_google_verifier(mock_verifier)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(&db).await?;
    let txn = shared.transaction();

    let user = backend::repos::users::create_user(txn, "user", false)
        .await
        .map_err(backend::AppError::from)?;
    backend::repos::auth_identities::create_identity(
        txn,
        user.id,
        PROVIDER_GOOGLE,
        &test_google_sub,
        &test_email.to_lowercase(),
    )
    .await
    .map_err(backend::AppError::from)?;

    let app = create_test_app(state).with_prod_routes().build().await?;

    let login_data = json!({ "id_token": "test-token" });
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "existing linked user should log in without admission check"
    );

    Ok(())
}

#[actix_web::test]
async fn test_bootstrap_seeding_inserts_missing_rows() -> Result<(), Box<dyn std::error::Error>> {
    env::set_var(
        "ALLOWED_EMAILS",
        "bootstrap1@example.com,bootstrap2@example.com",
    );

    let state = test_state_builder()?.build().await?;
    let db = require_db(&state).expect("DB required");

    let count = allowed_emails::Entity::find().count(&db).await?;
    assert!(
        count >= 2,
        "bootstrap should have seeded at least 2 rows from ALLOWED_EMAILS"
    );

    env::remove_var("ALLOWED_EMAILS");
    Ok(())
}

#[actix_web::test]
async fn test_bootstrap_seeding_additive_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let unique_email = backend_test_support::unique_helpers::unique_email("idem");
    let state = test_state_builder()?.build().await?;

    env::set_var("ALLOWED_EMAILS", &unique_email);

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let first = backend::repos::allowed_emails::seed_from_env(txn).await?;
            let second = backend::repos::allowed_emails::seed_from_env(txn).await?;
            assert!(first >= 1, "first run should insert");
            assert_eq!(second, 0, "second run should insert nothing (idempotent)");
            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    env::remove_var("ALLOWED_EMAILS");
    Ok(())
}

#[actix_web::test]
async fn test_repeated_first_login_no_duplicate_users() -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("repeated");
    let test_google_sub = unique_str("google");
    let mock_verifier = Arc::new(MockGoogleVerifier::new(VerifiedGoogleClaims {
        sub: test_google_sub.clone(),
        email: test_email.clone(),
        name: None,
    }));

    let state = test_state_builder()?
        .with_security(SecurityConfig::new(
            "test_secret_key_for_testing_purposes_only".as_bytes(),
        ))
        .with_google_verifier(mock_verifier)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    seed_admission_email(&db, &test_email.to_lowercase()).await;

    let shared = SharedTxn::open(&db).await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let login_data = json!({ "id_token": "test-token" });
    let req1 = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data.clone())
        .to_request();
    req1.extensions_mut().insert(shared.clone());
    let resp1 = test::call_service(&app, req1).await;
    assert!(resp1.status().is_success(), "first login should succeed");

    let req2 = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();
    req2.extensions_mut().insert(shared.clone());
    let resp2 = test::call_service(&app, req2).await;
    assert!(resp2.status().is_success(), "second login should succeed");

    let identity_count = backend::entities::user_auth_identities::Entity::find()
        .filter(backend::entities::user_auth_identities::Column::Provider.eq(PROVIDER_GOOGLE))
        .filter(
            backend::entities::user_auth_identities::Column::ProviderUserId.eq(&test_google_sub),
        )
        .count(shared.transaction())
        .await?;
    assert_eq!(
        identity_count, 1,
        "should have exactly one identity despite repeated logins"
    );

    Ok(())
}
