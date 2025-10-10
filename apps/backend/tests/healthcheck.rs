mod common;
mod support;

use actix_web::test;
use backend::infra::state::build_state;
use serde_json::Value;
use support::app_builder::create_test_app;

#[actix_web::test]
async fn test_health_endpoint() -> Result<(), Box<dyn std::error::Error>> {
    // Build state, then app using the two-stage harness
    let state = build_state().build().await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(resp.status().as_u16(), 200);

    let body = test::read_body(resp).await;
    let json: Value = serde_json::from_slice(&body)?;

    // Verify required fields are present
    assert_eq!(json["status"], "ok");
    assert_eq!(json["app_version"], "0.1.0");
    assert!(json["db"].is_string());
    assert!(json["migrations"].is_string());
    assert!(json["time"].is_string());

    // db field should be either "ok" or "error"
    let db_status = json["db"].as_str().unwrap();
    assert!(db_status == "ok" || db_status == "error");

    Ok(())
}
