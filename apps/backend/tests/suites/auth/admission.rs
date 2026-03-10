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
use backend::state::admission_mode::AdmissionMode;
use backend::state::security_config::SecurityConfig;
use backend_test_support::unique_helpers::{unique_email, unique_str};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::json;

use crate::common::assert_problem_details_structure;
use crate::support::app_builder::create_test_app;
use crate::support::auth::seed_admission_email;
use crate::support::test_state_builder;

const PROVIDER_GOOGLE: &str = "google";

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
        .with_admission_mode(AdmissionMode::Open)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    seed_admission_email(&db, &test_email.to_lowercase(), false).await;

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
        .with_admission_mode(AdmissionMode::Restricted)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    let other_email = backend_test_support::unique_helpers::unique_email("other");
    // Clear table so bootstrap ALLOWED_EMAILS cannot admit test_email; insert only other_email
    allowed_emails::Entity::delete_many().exec(&db).await?;
    seed_admission_email(&db, &other_email, false).await;

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
    let e1 = unique_email("bootstrap1");
    let e2 = unique_email("bootstrap2");
    env::set_var("ALLOWED_EMAILS", format!("{e1},{e2}"));

    let state = test_state_builder()?.build().await?;
    let db = require_db(&state).expect("DB required");

    let shared = SharedTxn::open(&db).await?;
    let _ = backend::repos::allowed_emails::seed_from_env(shared.transaction())
        .await
        .map_err(backend::AppError::from)?;

    let count = allowed_emails::Entity::find()
        .count(shared.transaction())
        .await?;
    assert!(
        count >= 2,
        "seed_from_env with 2 emails in ALLOWED_EMAILS should produce at least 2 rows (bootstrap or our seed)"
    );

    env::remove_var("ALLOWED_EMAILS");
    shared.rollback().await?;
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
        .with_admission_mode(AdmissionMode::Open)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    seed_admission_email(&db, &test_email.to_lowercase(), false).await;

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

#[actix_web::test]
async fn test_admission_seed_admin_from_env_wildcard_ignored(
) -> Result<(), Box<dyn std::error::Error>> {
    env::set_var("ALLOWED_EMAILS", "*@example.test");
    env::set_var("ADMIN_EMAILS", "*@example.test");

    let state = test_state_builder()?.build().await?;
    let db = require_db(&state).expect("DB required");

    let shared = SharedTxn::open(&db).await?;
    let _ = backend::repos::allowed_emails::seed_from_env(shared.transaction()).await?;
    let _ = backend::repos::allowed_emails::seed_admin_from_env(shared.transaction()).await?;

    let rows = allowed_emails::Entity::find()
        .filter(allowed_emails::Column::Email.eq("*@example.test"))
        .all(shared.transaction())
        .await?;
    assert_eq!(rows.len(), 1);
    assert!(
        !rows[0].is_admin,
        "wildcard in ADMIN_EMAILS must not be marked admin"
    );

    env::remove_var("ALLOWED_EMAILS");
    env::remove_var("ADMIN_EMAILS");
    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn test_admission_seed_admin_from_env_exact_marks_or_creates(
) -> Result<(), Box<dyn std::error::Error>> {
    let exact = unique_email("admin-exact");
    env::set_var("ALLOWED_EMAILS", &exact);
    env::set_var("ADMIN_EMAILS", &exact);

    let state = test_state_builder()?.build().await?;
    let db = require_db(&state).expect("DB required");

    let rows = allowed_emails::Entity::find()
        .filter(allowed_emails::Column::Email.eq(exact.to_lowercase()))
        .all(&db)
        .await?;
    assert_eq!(
        rows.len(),
        1,
        "bootstrap should seed exact email from ALLOWED_EMAILS"
    );
    assert!(rows[0].is_admin, "exact admin email must be marked admin");

    env::remove_var("ALLOWED_EMAILS");
    env::remove_var("ADMIN_EMAILS");
    Ok(())
}

#[actix_web::test]
async fn test_admission_seed_admin_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let exact = unique_email("admin-idem");
    env::set_var("ALLOWED_EMAILS", &exact);
    env::set_var("ADMIN_EMAILS", &exact);

    let state = test_state_builder()?.build().await?;
    let db = require_db(&state).expect("DB required");

    let shared = SharedTxn::open(&db).await?;
    let _ = backend::repos::allowed_emails::seed_from_env(shared.transaction()).await?;
    let _ = backend::repos::allowed_emails::seed_admin_from_env(shared.transaction()).await?;
    let _ = backend::repos::allowed_emails::seed_admin_from_env(shared.transaction()).await?;

    let count = allowed_emails::Entity::find()
        .filter(allowed_emails::Column::Email.eq(exact.to_lowercase()))
        .count(shared.transaction())
        .await?;
    assert_eq!(count, 1, "repeated seed must not create duplicate rows");

    env::remove_var("ALLOWED_EMAILS");
    env::remove_var("ADMIN_EMAILS");
    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn test_open_mode_allows_first_time_login_even_with_admin_rows(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("open-mode-user");
    let test_google_sub = unique_str("google");
    let mock_verifier = Arc::new(MockGoogleVerifier::new(VerifiedGoogleClaims {
        sub: test_google_sub.clone(),
        email: test_email.clone(),
        name: Some("Open Mode User".to_string()),
    }));

    let state = test_state_builder()?
        .with_security(SecurityConfig::new(
            "test_secret_key_for_testing_purposes_only".as_bytes(),
        ))
        .with_google_verifier(mock_verifier)
        .with_admission_mode(AdmissionMode::Open)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    allowed_emails::Entity::delete_many().exec(&db).await?;
    seed_admission_email(&db, "admin@example.com", true).await;

    let app = create_test_app(state).with_prod_routes().build().await?;

    let login_data = json!({ "id_token": "test-token" });
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "open mode should allow first-time login even when only admin rows exist"
    );

    Ok(())
}

#[actix_web::test]
async fn test_restricted_mode_requires_admission_match() -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("restricted-denied");
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
        .with_admission_mode(AdmissionMode::Restricted)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required");
    allowed_emails::Entity::delete_many().exec(&db).await?;
    seed_admission_email(&db, "other@example.com", false).await;

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
async fn test_admin_only_seeded_rows_do_not_flip_open_to_restricted(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_email = unique_email("admin-only-open");
    let test_google_sub = unique_str("google");
    let mock_verifier = Arc::new(MockGoogleVerifier::new(VerifiedGoogleClaims {
        sub: test_google_sub,
        email: test_email.clone(),
        name: None,
    }));

    env::remove_var("ALLOWED_EMAILS");
    env::set_var("ADMIN_EMAILS", "admin@example.com");

    let state = test_state_builder()?
        .with_security(SecurityConfig::new(
            "test_secret_key_for_testing_purposes_only".as_bytes(),
        ))
        .with_google_verifier(mock_verifier)
        .with_admission_mode(AdmissionMode::Open)
        .build()
        .await?;

    backend::db::txn::with_txn(None, &state, |txn| {
        Box::pin(async move {
            let _ = backend::repos::allowed_emails::seed_admin_from_env(txn)
                .await
                .map_err(backend::AppError::from)?;
            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    let app = create_test_app(state).with_prod_routes().build().await?;

    let login_data = json!({ "id_token": "test-token" });
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "open mode with admin-only rows should still allow any email (admission from config, not table)"
    );

    env::remove_var("ADMIN_EMAILS");
    Ok(())
}

#[actix_web::test]
async fn test_admission_seed_normalization_case_admin() -> Result<(), Box<dyn std::error::Error>> {
    let base = unique_email("norm-admin");
    env::set_var("ALLOWED_EMAILS", &base);
    env::set_var("ADMIN_EMAILS", base.to_uppercase());

    let state = test_state_builder()?.build().await?;
    let db = require_db(&state).expect("DB required");

    let shared = SharedTxn::open(&db).await?;
    let _ = backend::repos::allowed_emails::seed_from_env(shared.transaction()).await?;
    let _ = backend::repos::allowed_emails::seed_admin_from_env(shared.transaction()).await?;

    let normalized = base.to_lowercase();
    let count = allowed_emails::Entity::find()
        .filter(allowed_emails::Column::Email.eq(&normalized))
        .count(shared.transaction())
        .await?;
    assert_eq!(
        count, 1,
        "different casing must produce single normalized row"
    );

    let row = allowed_emails::Entity::find()
        .filter(allowed_emails::Column::Email.eq(&normalized))
        .one(shared.transaction())
        .await?
        .expect("row must exist");
    assert!(
        row.is_admin,
        "admin from ADMIN_EMAILS with different casing must be marked admin"
    );

    env::remove_var("ALLOWED_EMAILS");
    env::remove_var("ADMIN_EMAILS");
    shared.rollback().await?;
    Ok(())
}
