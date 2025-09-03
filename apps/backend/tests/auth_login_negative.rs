use actix_web::test;
use backend::{
    bootstrap::db::DbProfile,
    test_support::{build_state, create_test_app},
};
use serde_json::json;

#[actix_web::test]
async fn login_rejects_empty_fields_returns_problem_details(
) -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and default security config
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Build app with production routes
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

    // Test empty email
    let login_data_empty_email = json!({
        "email": "",
        "google_sub": "google_123",
        "name": "Test User"
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data_empty_email)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return a 400 Bad Request for empty email
    assert_eq!(resp.status().as_u16(), 400);

    // Verify it returns Problem Details format
    let content_type = resp.headers().get("content-type").unwrap();
    assert!(content_type
        .to_str()
        .unwrap()
        .contains("application/problem+json"));

    // Verify Problem Details structure
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("type").is_some());
    assert!(body.get("title").is_some());
    assert_eq!(body["status"], 400);
    assert!(body.get("detail").is_some());
    assert!(body.get("code").is_some());
    assert!(body.get("trace_id").is_some());

    // Verify code is SCREAMING_SNAKE
    let code = body["code"].as_str().unwrap();
    assert!(
        code.chars().all(|c| c.is_uppercase() || c == '_'),
        "Code should be SCREAMING_SNAKE_CASE"
    );
    assert_eq!(code, "INVALID_EMAIL");

    // Test empty google_sub
    let login_data_empty_google_sub = json!({
        "email": "test@example.com",
        "google_sub": "",
        "name": "Test User"
    });

    let req2 = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data_empty_google_sub)
        .to_request();

    let resp2 = test::call_service(&app, req2).await;

    // Should return a 400 Bad Request for empty google_sub
    assert_eq!(resp2.status().as_u16(), 400);

    let content_type2 = resp2.headers().get("content-type").unwrap();
    assert!(content_type2
        .to_str()
        .unwrap()
        .contains("application/problem+json"));

    let body2: serde_json::Value = test::read_body_json(resp2).await;
    assert_eq!(body2["status"], 400);
    assert_eq!(body2["code"], "INVALID_GOOGLE_SUB");

    // Test both empty
    let login_data_both_empty = json!({
        "email": "",
        "google_sub": "",
        "name": ""
    });

    let req3 = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data_both_empty)
        .to_request();

    let resp3 = test::call_service(&app, req3).await;

    // Should return a 400 Bad Request
    assert_eq!(resp3.status().as_u16(), 400);

    let body3: serde_json::Value = test::read_body_json(resp3).await;
    assert_eq!(body3["status"], 400);
    // Should fail on first validation (email)
    assert_eq!(body3["code"], "INVALID_EMAIL");

    Ok(())
}

#[actix_web::test]
async fn login_missing_email_returns_400_todo_validator() -> Result<(), Box<dyn std::error::Error>>
{
    // Build state with database and default security config
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Build app with production routes
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

    // Test missing email field entirely
    let login_data_missing_email = json!({
        "google_sub": "google_123",
        "name": "Test User"
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data_missing_email)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return a 400 Bad Request
    assert_eq!(resp.status().as_u16(), 400);

    // TODO: Upgrade this assertion once `ValidatedJson<T>` is implemented.
    // For now, we expect a 400 but don't assert Problem Details shape since serde fails before handler.

    Ok(())
}

#[actix_web::test]
async fn login_missing_google_sub_returns_400_todo_validator(
) -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and default security config
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Build app with production routes
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

    // Test missing google_sub field entirely
    let login_data_missing_google_sub = json!({
        "email": "test@example.com",
        "name": "Test User"
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data_missing_google_sub)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return a 400 Bad Request
    assert_eq!(resp.status().as_u16(), 400);

    // TODO: Upgrade this assertion once `ValidatedJson<T>` is implemented.
    // For now, we expect a 400 but don't assert Problem Details shape since serde fails before handler.

    Ok(())
}

#[actix_web::test]
async fn login_wrong_type_returns_400_todo_validator() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and default security config
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Build app with production routes
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

    // Test wrong type for email (number instead of string)
    let login_data_wrong_email_type = json!({
        "email": 123,
        "google_sub": "google_123",
        "name": "Test User"
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data_wrong_email_type)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return a 400 Bad Request
    assert_eq!(resp.status().as_u16(), 400);

    // TODO: Upgrade this assertion once `ValidatedJson<T>` is implemented.
    // For now, we expect a 400 but don't assert Problem Details shape since serde fails before handler.

    // Test wrong type for google_sub (number instead of string)
    let login_data_wrong_google_sub_type = json!({
        "email": "test@example.com",
        "google_sub": 456,
        "name": "Test User"
    });

    let req2 = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data_wrong_google_sub_type)
        .to_request();

    let resp2 = test::call_service(&app, req2).await;

    // Should return a 400 Bad Request
    assert_eq!(resp2.status().as_u16(), 400);

    // TODO: Upgrade this assertion once `ValidatedJson<T>` is implemented.
    // For now, we expect a 400 but don't assert Problem Details shape since serde fails before handler.

    // Test wrong type for name (number instead of string)
    let login_data_wrong_name_type = json!({
        "email": "test@example.com",
        "google_sub": "google_123",
        "name": 789
    });

    let req3 = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data_wrong_name_type)
        .to_request();

    let resp3 = test::call_service(&app, req3).await;

    // Should return a 400 Bad Request
    assert_eq!(resp3.status().as_u16(), 400);

    // TODO: Upgrade this assertion once `ValidatedJson<T>` is implemented.
    // For now, we expect a 400 but don't assert Problem Details shape since serde fails before handler.

    Ok(())
}
