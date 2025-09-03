use actix_web::test;
use backend::test_support::{build_state, create_test_app};

#[actix_web::test]
async fn test_health_endpoint() -> Result<(), Box<dyn std::error::Error>> {
    // Build state, then app using the two-stage harness
    let state = build_state().build().await?;
    let app = create_test_app(state.clone())
        .with_prod_routes()
        .build()
        .await?;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(resp.status().as_u16(), 200);

    let body = test::read_body(resp).await;
    assert_eq!(body, "ok");

    Ok(())
}
