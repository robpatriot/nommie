// Tests for rate limiting middleware
//
// Verifies that rate limits are enforced correctly on different route groups.

use std::time::Duration;

use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_extensible_rate_limit::backend::SimpleInputFunctionBuilder;
use actix_extensible_rate_limit::RateLimiter;
use actix_web::{test, web, App, HttpResponse, Result};
use backend::middleware::request_trace::RequestTrace;
use backend::middleware::structured_logger::StructuredLogger;
use backend::middleware::trace_span::TraceSpan;

/// Simple test handler that returns 200 OK
async fn test_handler() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "ok"})))
}

#[actix_web::test]
async fn test_rate_limit_enforces_limit() -> Result<(), Box<dyn std::error::Error>> {
    // Use a low limit (2 requests) with a 1-second window for fast testing
    let backend = InMemoryBackend::builder().build();
    let input = SimpleInputFunctionBuilder::new(Duration::from_secs(1), 2)
        .path_key()
        .build();
    let rate_limiter = RateLimiter::builder(backend, input).add_headers().build();

    let app = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .wrap(TraceSpan)
            .wrap(RequestTrace)
            .wrap(rate_limiter)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // First two requests should succeed
    for i in 0..2 {
        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;

        assert!(
            resp.status().is_success(),
            "Request {} should succeed (within rate limit)",
            i + 1
        );
        assert_eq!(
            resp.status().as_u16(),
            200,
            "Request {} should return 200 OK",
            i + 1
        );

        // Verify rate limit headers are present
        assert!(
            resp.headers().contains_key("x-ratelimit-remaining"),
            "Request {} should include rate limit headers",
            i + 1
        );
    }

    // Third request should be rate limited (429)
    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(
        resp.status().as_u16(),
        429,
        "Third request should be rate limited (429 Too Many Requests)"
    );

    Ok(())
}

#[actix_web::test]
async fn test_rate_limit_resets_after_window() -> Result<(), Box<dyn std::error::Error>> {
    // Use a limit of 1 request with a very short window to keep the test fast
    let backend = InMemoryBackend::builder().build();
    let input = SimpleInputFunctionBuilder::new(Duration::from_millis(10), 1)
        .path_key()
        .build();
    let rate_limiter = RateLimiter::builder(backend, input).add_headers().build();

    let app = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .wrap(TraceSpan)
            .wrap(RequestTrace)
            .wrap(rate_limiter)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // First request should succeed
    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    // Second request immediately should be rate limited
    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 429);

    // Wait for the rate limit window to expire (with a small buffer)
    tokio::time::sleep(Duration::from_millis(25)).await;

    // Request after window should succeed again
    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status().as_u16(),
        200,
        "Request after rate limit window should succeed"
    );

    Ok(())
}

#[actix_web::test]
async fn test_rate_limit_headers_are_present() -> Result<(), Box<dyn std::error::Error>> {
    let backend = InMemoryBackend::builder().build();
    let input = SimpleInputFunctionBuilder::new(Duration::from_secs(60), 5)
        .path_key()
        .build();
    let rate_limiter = RateLimiter::builder(backend, input).add_headers().build();

    let app = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .wrap(TraceSpan)
            .wrap(RequestTrace)
            .wrap(rate_limiter)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert!(
        resp.headers().contains_key("x-ratelimit-remaining"),
        "Response should include x-ratelimit-remaining header"
    );
    assert!(
        resp.headers().contains_key("x-ratelimit-limit"),
        "Response should include x-ratelimit-limit header"
    );

    Ok(())
}
