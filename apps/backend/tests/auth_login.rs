use actix_web::test;
use backend::{
    bootstrap::db::DbProfile,
    state::SecurityConfig,
    test_support::{build_state, create_test_app},
};
use serde_json::json;

#[actix_web::test]
async fn test_login_endpoint_create_and_reuse_user() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and custom security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config.clone())
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

    // Test 1: First login with new email -> creates user + returns JWT
    let login_data = json!({
        "email": "test@example.com",
        "name": "Test User",
        "google_sub": "google_123"
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("token").is_some());

    let token = body["token"].as_str().unwrap();
    assert!(!token.is_empty());

    // Test 2: Decode the JWT to verify correct sub and email
    let decoded =
        backend::verify_access_token(token, &security_config).expect("JWT should be valid");
    assert_eq!(decoded.email, "test@example.com");

    // Store the user sub from first login
    let first_user_sub = decoded.sub;

    // Test 3: Second call with the same email -> reuses the same user
    let login_data_2 = json!({
        "email": "test@example.com",
        "name": "Updated Name", // Different name shouldn't matter
        "google_sub": "google_456" // Different google_sub shouldn't matter
    });

    let req2 = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data_2)
        .to_request();

    let resp2 = test::call_service(&app, req2).await;

    assert!(resp2.status().is_success());
    assert_eq!(resp2.status().as_u16(), 200);

    let body2: serde_json::Value = test::read_body_json(resp2).await;
    let token2 = body2["token"].as_str().unwrap();

    let decoded2 =
        backend::verify_access_token(token2, &security_config).expect("JWT should be valid");

    // Verify that the same user sub is returned (user was reused)
    assert_eq!(decoded2.sub, first_user_sub);
    assert_eq!(decoded2.email, "test@example.com");

    Ok(())
}

#[actix_web::test]
async fn test_login_endpoint_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with database and custom security config
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_db(DbProfile::Test)
        .with_security(security_config.clone())
        .build()
        .await?;

    // Build app with production routes
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

    // Test missing required fields
    let login_data = json!({
        "email": "test@example.com"
        // Missing google_sub and name is optional
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return a 400 Bad Request due to missing google_sub
    assert!(resp.status().is_client_error());

    // Verify it returns Problem Details format
    let content_type = resp.headers().get("content-type").unwrap();
    assert!(content_type
        .to_str()
        .unwrap()
        .contains("application/problem+json"));

    // Test with empty email
    let login_data_empty_email = json!({
        "email": "",
        "google_sub": "google_123"
    });

    let req2 = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(login_data_empty_email)
        .to_request();

    let resp2 = test::call_service(&app, req2).await;

    // Should return a 400 Bad Request for empty email
    assert!(resp2.status().is_client_error());

    let content_type2 = resp2.headers().get("content-type").unwrap();
    assert!(content_type2
        .to_str()
        .unwrap()
        .contains("application/problem+json"));

    Ok(())
}
