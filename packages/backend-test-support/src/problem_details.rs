//! Problem Details test helpers for backend testing
//!
//! This module provides utilities for asserting Problem Details (RFC7807) error
//! responses in both unit and integration tests without depending on backend types.
//!
//! What we validate by default (stronger than before):
//! - HTTP status matches the expected value
//! - Content-Type is `application/problem+json` (charset tolerated)
//! - `x-trace-id` header exists, is non-empty, and matches a non-empty `trace_id` in the body
//! - For 401: `WWW-Authenticate: Bearer` present, `Retry-After` absent
//! - For 429/503: optional `Retry-After` is validated (delta-seconds or HTTP-date), `WWW-Authenticate` absent
//! - For other 4xx/5xx: both `WWW-Authenticate` and `Retry-After` absent
//! - `type` and `title` are non-empty strings
//! - `code` equals expected
//! - `status` in body equals HTTP status
//! - Optional substring requirement in `detail`

use actix_web::http::{
    header::{HeaderMap, HeaderName, CONTENT_TYPE, RETRY_AFTER, WWW_AUTHENTICATE},
    StatusCode,
};
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

fn header_name_x_trace_id() -> HeaderName {
    // Using HeaderName constructor allows us to avoid hard-coding casing.
    HeaderName::from_static("x-trace-id")
}

fn assert_content_type_problem_json(headers: &HeaderMap) {
    let ct = headers
        .get(CONTENT_TYPE)
        .expect("content-type header should be present")
        .to_str()
        .expect("content-type header must be valid UTF-8");
    let mime = ct.split(';').next().unwrap_or("").trim();
    assert!(
        mime.eq_ignore_ascii_case("application/problem+json"),
        "content-type must be application/problem+json (got: {ct})"
    );
}

fn looks_like_http_date(s: &str) -> bool {
    // Loosely accept common HTTP-date shapes without pulling in a parser:
    // e.g. "Mon, 02 Jan 2006 15:04:05 GMT"
    s.contains(',') && s.contains("GMT")
}

fn assert_header_semantics(headers: &HeaderMap, status: StatusCode) {
    let has_www = headers.get(WWW_AUTHENTICATE).is_some();
    let retry_after = headers.get(RETRY_AFTER).map(|v| {
        v.to_str()
            .expect("Retry-After must be valid UTF-8")
            .trim()
            .to_string()
    });

    match status.as_u16() {
        401 => {
            let wa = headers
                .get(WWW_AUTHENTICATE)
                .expect("WWW-Authenticate must be present for 401 responses")
                .to_str()
                .expect("WWW-Authenticate must be valid UTF-8");
            assert_eq!(
                wa, "Bearer",
                "WWW-Authenticate must be 'Bearer' for 401 responses (got: {wa})"
            );
            assert!(
                retry_after.is_none(),
                "Retry-After should not be present for 401 responses"
            );
        }
        429 | 503 => {
            // Retry-After is optional; if present validate shape (delta-seconds or HTTP-date)
            if let Some(val) = retry_after {
                let is_int = val.parse::<u64>().is_ok();
                let is_date = looks_like_http_date(&val);
                assert!(
                    is_int || is_date,
                    "Retry-After must be delta-seconds or an HTTP-date (got: {val})"
                );
            }
            assert!(
                !has_www,
                "WWW-Authenticate should not be present for {status} responses"
            );
        }
        _ => {
            // For other 4xx/5xx we expect neither of these control headers
            if status.is_client_error() || status.is_server_error() {
                assert!(
                    !has_www,
                    "WWW-Authenticate should not be present for {status} responses"
                );
                assert!(
                    retry_after.is_none(),
                    "Retry-After should not be present for {status} responses"
                );
            }
        }
    }
}

/// Assert that an HTTP response conforms to the stable error contract
///
/// Validates:
/// - HTTP status matches expected
/// - Content-Type is application/problem+json
/// - x-trace-id header exists, is non-empty, and matches body trace_id (also non-empty)
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
    let body = actix_web::body::to_bytes(body_bytes)
        .await
        .expect("failed to read response body");

    assert_problem_details_from_parts(
        status,
        &headers,
        &body,
        expected_code,
        expected_status,
        expected_detail_contains,
    )
    .await;
}

/// Core validator operating on raw response parts (used by both convenience wrappers)
pub async fn assert_problem_details_from_parts(
    status: StatusCode,
    headers: &HeaderMap,
    body_bytes: &[u8],
    expected_code: &str,
    expected_status: StatusCode,
    expected_detail_contains: Option<&str>,
) {
    // 1) Status
    assert_eq!(status, expected_status, "unexpected HTTP status");

    // 2) Content-Type
    assert_content_type_problem_json(headers);

    // 3) Parse Problem Details
    let body_str =
        String::from_utf8(body_bytes.to_vec()).expect("Response body should be valid UTF-8");
    let problem: ProblemDetailsLike =
        serde_json::from_str(&body_str).expect("Response body must be ProblemDetails JSON");

    // 4) Trace ID parity + non-empty
    let trace_id_header = headers
        .get(header_name_x_trace_id())
        .expect("x-trace-id header should be present")
        .to_str()
        .expect("x-trace-id header should be valid UTF-8");
    assert!(
        !trace_id_header.is_empty(),
        "x-trace-id header must not be empty"
    );
    assert!(
        !problem.trace_id.is_empty(),
        "trace_id in body must not be empty"
    );
    assert_eq!(
        problem.trace_id, trace_id_header,
        "trace_id in body should match x-trace-id header"
    );

    // 5) Header semantics (WWW-Authenticate / Retry-After rules)
    assert_header_semantics(headers, status);

    // 6) Contract fields
    assert_eq!(problem.code, expected_code, "unexpected error code");
    assert_eq!(
        problem.status,
        expected_status.as_u16(),
        "status in body must match HTTP status"
    );

    // 7) Require non-empty type/title
    assert!(
        !problem.type_.is_empty(),
        "RFC7807 'type' must not be empty"
    );
    assert!(
        !problem.title.is_empty(),
        "RFC7807 'title' must not be empty"
    );

    // 8) Optional detail substring
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
    )
    .await;
}

/// Convenience wrapper used in tests:
/// `assert_problem_details_structure(resp, 400, "INVALID_EMAIL", "Email cannot be empty").await;`
pub async fn assert_problem_details_structure(
    resp: actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
    expected_status: u16,
    expected_code: &str,
    expected_detail_contains: &str,
) {
    let status =
        StatusCode::from_u16(expected_status).expect("expected_status must be a valid StatusCode");
    assert_problem_details_from_service_response(
        resp,
        expected_code,
        status,
        Some(expected_detail_contains),
    )
    .await;
}
