//! Problem Details test helpers for backend testing
//!
//! This module provides utilities for asserting Problem Details responses
//! in both unit and integration tests without depending on backend types.

use actix_web::http::StatusCode;
use serde::{Deserialize, Serialize};

/// Local ProblemDetails struct that matches the backend's structure
/// but doesn't depend on backend types
#[derive(Debug, Deserialize, Serialize)]
struct ProblemDetailsLike {
    #[serde(rename = "type")]
    type_: String,
    title: String,
    status: u16,
    detail: String,
    code: String,
    trace_id: String,
}

/// Assert that an HTTP response conforms to the stable error contract
/// 
/// This helper operates on HttpResponse and validates:
/// - HTTP status matches expected
/// - x-trace-id header exists and matches body trace_id
/// - Problem Details fields match expected values
pub async fn assert_problem_details_from_http_response(
    resp: actix_web::HttpResponse,
    expected_code: &str,
    expected_status: StatusCode,
    expected_detail_contains: Option<&str>,
) {
    let status = resp.status();
    let headers = resp.headers().clone();
    
    // Convert HttpResponse to bytes for parsing
    let body_bytes = resp.into_body();
    let body = actix_web::body::to_bytes(body_bytes).await.unwrap();
    
    assert_problem_details_from_parts(
        status,
        &headers,
        &body,
        expected_code,
        expected_status,
        expected_detail_contains,
    ).await;
}

/// Assert that response parts conform to the stable error contract
/// 
/// This helper operates on raw response parts and validates:
/// - HTTP status matches expected
/// - x-trace-id header exists and matches body trace_id
/// - Problem Details fields match expected values
pub async fn assert_problem_details_from_parts(
    status: StatusCode,
    headers: &actix_web::http::header::HeaderMap,
    body_bytes: &[u8],
    expected_code: &str,
    expected_status: StatusCode,
    expected_detail_contains: Option<&str>,
) {
    // Assert HTTP status matches expected
    assert_eq!(status, expected_status);
    
    // Parse the response body as ProblemDetails
    let body_str = String::from_utf8(body_bytes.to_vec())
        .expect("Response body should be valid UTF-8");
    let problem: ProblemDetailsLike = serde_json::from_str(&body_str)
        .expect("Response body should be valid ProblemDetails JSON");
    
    // Assert trace_id parity: body trace_id should equal x-trace-id header
    let trace_id_header = headers
        .get("x-trace-id")
        .expect("x-trace-id header should be present")
        .to_str()
        .expect("x-trace-id header should be valid UTF-8");
    
    assert_eq!(
        problem.trace_id, trace_id_header,
        "trace_id in body should match x-trace-id header"
    );

    // Assert the contract fields
    assert_eq!(problem.code, expected_code);
    assert_eq!(problem.status, expected_status.as_u16());

    // Assert detail substring if provided
    if let Some(expected_detail) = expected_detail_contains {
        assert!(
            problem.detail.contains(expected_detail),
            "Expected detail to contain '{}', but got '{}'",
            expected_detail,
            problem.detail
        );
    }
}

/// Assert that a ServiceResponse conforms to the stable error contract
/// 
/// This helper operates on ServiceResponse<BoxBody> and validates:
/// - HTTP status matches expected
/// - x-trace-id header exists and matches body trace_id
/// - Problem Details fields match expected values
pub async fn assert_problem_details_from_service_response(
    resp: actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
    expected_code: &str,
    expected_status: StatusCode,
    expected_detail_contains: Option<&str>,
) {
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = actix_web::test::read_body(resp).await;
    
    assert_problem_details_from_parts(
        status,
        &headers,
        &body,
        expected_code,
        expected_status,
        expected_detail_contains,
    ).await;
}
