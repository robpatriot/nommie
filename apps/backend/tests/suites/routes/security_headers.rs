// Tests for security headers middleware
//
// Verifies that security headers are correctly applied to all endpoints
// and that Cache-Control: no-store is conditionally applied.

use actix_web::{test, web, App, HttpResponse, Result};
use backend::middleware::request_trace::RequestTrace;
use backend::middleware::security_headers::SecurityHeaders;
use backend::middleware::structured_logger::StructuredLogger;
use backend::middleware::trace_span::TraceSpan;

/// Simple test handler that returns 200 OK
async fn test_handler() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "ok"})))
}

/// Simple root handler
async fn root_handler() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("Hello"))
}

#[actix_web::test]
async fn test_security_headers_present_on_api_endpoint() -> Result<(), Box<dyn std::error::Error>> {
    let app = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .wrap(TraceSpan)
            .wrap(RequestTrace)
            .wrap(SecurityHeaders)
            .route("/api/test", web::get().to(test_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/test").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    let headers = resp.headers();

    // Verify all security headers are present
    assert!(
        headers.contains_key("x-content-type-options"),
        "Response should include X-Content-Type-Options header"
    );
    assert_eq!(
        headers.get("x-content-type-options").unwrap(),
        "nosniff",
        "X-Content-Type-Options should be 'nosniff'"
    );

    assert!(
        headers.contains_key("x-frame-options"),
        "Response should include X-Frame-Options header"
    );
    assert_eq!(
        headers.get("x-frame-options").unwrap(),
        "DENY",
        "X-Frame-Options should be 'DENY'"
    );

    assert!(
        headers.contains_key("strict-transport-security"),
        "Response should include Strict-Transport-Security header"
    );
    assert_eq!(
        headers.get("strict-transport-security").unwrap(),
        "max-age=31536000; includeSubDomains",
        "Strict-Transport-Security should have correct value"
    );

    assert!(
        headers.contains_key("referrer-policy"),
        "Response should include Referrer-Policy header"
    );
    assert_eq!(
        headers.get("referrer-policy").unwrap(),
        "strict-origin-when-cross-origin",
        "Referrer-Policy should be 'strict-origin-when-cross-origin'"
    );

    // Verify Cache-Control: no-store is present for API endpoints
    assert!(
        headers.contains_key("cache-control"),
        "API response should include Cache-Control header"
    );
    assert_eq!(
        headers.get("cache-control").unwrap(),
        "no-store",
        "Cache-Control should be 'no-store' for API endpoints"
    );

    Ok(())
}

#[actix_web::test]
async fn test_security_headers_present_on_health_endpoint() -> Result<(), Box<dyn std::error::Error>>
{
    let app = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .wrap(TraceSpan)
            .wrap(RequestTrace)
            .wrap(SecurityHeaders)
            .route("/health", web::get().to(test_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    let headers = resp.headers();

    // Verify security headers are present
    assert!(headers.contains_key("x-content-type-options"));
    assert!(headers.contains_key("x-frame-options"));
    assert!(headers.contains_key("strict-transport-security"));
    assert!(headers.contains_key("referrer-policy"));

    // Verify Cache-Control: no-store is present for health endpoint
    assert!(
        headers.contains_key("cache-control"),
        "Health response should include Cache-Control header"
    );
    assert_eq!(
        headers.get("cache-control").unwrap(),
        "no-store",
        "Cache-Control should be 'no-store' for health endpoint"
    );

    Ok(())
}

#[actix_web::test]
async fn test_security_headers_present_on_root_no_cache_control(
) -> Result<(), Box<dyn std::error::Error>> {
    let app = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .wrap(TraceSpan)
            .wrap(RequestTrace)
            .wrap(SecurityHeaders)
            .route("/", web::get().to(root_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    let headers = resp.headers();

    // Verify security headers are present
    assert!(headers.contains_key("x-content-type-options"));
    assert!(headers.contains_key("x-frame-options"));
    assert!(headers.contains_key("strict-transport-security"));
    assert!(headers.contains_key("referrer-policy"));

    // Verify Cache-Control: no-store is NOT present for root endpoint
    assert!(
        !headers.contains_key("cache-control"),
        "Root endpoint should NOT include Cache-Control: no-store header"
    );

    Ok(())
}
