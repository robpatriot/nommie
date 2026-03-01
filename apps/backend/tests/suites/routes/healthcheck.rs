use actix_web::test;
use backend::infra::state::build_state;
use serde_json::Value;

use crate::support::app_builder::create_test_app;

#[actix_web::test]
async fn test_public_health_probes() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state().build().await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let req = test::TestRequest::get().uri("/api/livez").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "alive");

    let req = test::TestRequest::get().uri("/api/readyz").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 503);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "not_ready");
    assert_eq!(body["ready"], false);

    Ok(())
}

#[actix_web::test]
async fn test_internal_readiness_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_state().build().await?;
    let app = create_test_app(state).with_prod_routes().build().await?;

    let req = test::TestRequest::get()
        .uri("/api/internal/readyz")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 503);

    let json: Value = test::read_body_json(resp).await;

    assert_eq!(json["service"], "backend");
    assert!(json["uptime_seconds"].is_number());
    assert_eq!(json["state"]["mode"], "startup");
    assert_eq!(json["state"]["ready"], false);
    assert!(json["dependencies"].is_array());
    assert!(json["migration"].is_object());

    Ok(())
}
