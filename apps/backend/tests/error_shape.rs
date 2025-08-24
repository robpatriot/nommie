use actix_web::{test, web, App, HttpResponse, HttpRequest, HttpMessage};
use backend::{AppError, middleware::RequestTrace};

async fn test_error_handler(req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::invalid("INVALID_EXAMPLE", "Example failure".to_string())
        .with_trace_id(req.extensions().get::<String>().cloned()))
}

#[actix_web::test]
async fn test_error_shape() {
    // Create a minimal test app with RequestTrace middleware
    let app = test::init_service(
        App::new()
            .wrap(RequestTrace)
            .route("/_test/error", web::get().to(test_error_handler))
    ).await;

    // Create a request to the test error endpoint
    let req = test::TestRequest::get().uri("/_test/error").to_request();
    let resp = test::call_service(&app, req).await;

    // Assert status code is 400 (Bad Request)
    assert_eq!(resp.status().as_u16(), 400);

    // Extract headers before reading body to avoid borrowing issues
    let headers = resp.headers().clone();
    let request_id_header = headers.get("x-request-id");
    assert!(request_id_header.is_some());
    let request_id = request_id_header.unwrap().to_str().unwrap();
    assert!(!request_id.is_empty());

    // Assert Content-Type is application/problem+json
    let content_type = headers.get("content-type").unwrap().to_str().unwrap();
    assert_eq!(content_type, "application/problem+json");

    // Read and parse the response body
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    let problem_details: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    // Assert all required keys are present
    assert!(problem_details.get("type").is_some());
    assert!(problem_details.get("title").is_some());
    assert!(problem_details.get("status").is_some());
    assert!(problem_details.get("detail").is_some());
    assert!(problem_details.get("code").is_some());
    assert!(problem_details.get("trace_id").is_some());

    // Assert specific values
    assert_eq!(problem_details["code"], "INVALID_EXAMPLE");
    assert_eq!(problem_details["detail"], "Example failure");
    assert_eq!(problem_details["status"], 400);

    // Assert trace_id in body equals the header value
    let trace_id_in_body = problem_details["trace_id"].as_str().unwrap();
    assert_eq!(trace_id_in_body, request_id);
}
