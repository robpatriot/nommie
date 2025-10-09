#![allow(dead_code)]

// tests/common/mod.rs
use actix_web::body::BoxBody;
use actix_web::dev::ServiceResponse;
use actix_web::http::header::{HeaderName, CONTENT_TYPE};
use actix_web::test;
use serde_json::Value;

// Logging is auto-installed for most test binaries
#[ctor::ctor]
fn init_logging() {
    backend_test_support::logging::init();
}

// Policy defaults to rollback but can be flipped per-binary via `NOMMIE_TXN_POLICY=commit`.
#[ctor::ctor]
fn init_txn_policy() {
    let policy = match std::env::var("NOMMIE_TXN_POLICY")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "commit" => backend::db::txn_policy::TxnPolicy::CommitOnOk,
        _ => backend::db::txn_policy::TxnPolicy::RollbackOnOk,
    };

    backend::db::txn_policy::set_txn_policy(policy);
}

/// Helper function to check that the trace_id in the response body matches the X-Trace-Id header
pub fn assert_trace_id_matches(json: &Value, header_trace_id: &str) {
    let trace_id_in_body = json["trace_id"]
        .as_str()
        .expect("trace_id field should be a string");
    assert_eq!(
        trace_id_in_body, header_trace_id,
        "trace_id in body should match X-Trace-Id header"
    );
}

/// Helper function to validate that a response follows the ProblemDetails structure
/// and that trace_id matches the X-Trace-Id header
pub async fn assert_problem_details_structure(
    resp: ServiceResponse<BoxBody>,
    expected_status: u16,
    expected_code: &str,
    expected_detail: &str,
) {
    // Assert status code
    assert_eq!(resp.status().as_u16(), expected_status);

    // Extract headers before consuming the response
    let headers = resp.headers().clone();

    // X-Trace-Id (header names are case-insensitive; use a typed HeaderName)
    let trace_hdr = HeaderName::from_static("x-trace-id");
    let trace_id = headers
        .get(&trace_hdr)
        .and_then(|v| v.to_str().ok())
        .expect("X-Trace-Id header should be present and valid UTF-8");
    assert!(
        !trace_id.is_empty(),
        "X-Trace-Id header should not be empty"
    );

    // Content-Type may include parameters (e.g., charset)
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(
        content_type.starts_with("application/problem+json"),
        "Content-Type must be application/problem+json (got {content_type})"
    );

    // Validate HTTP header rules (defined in src/error.rs error_response):
    // - 401: WWW-Authenticate: Bearer (no Retry-After)
    // - 503: Retry-After (no WWW-Authenticate)
    // - 400/404/409: neither header
    match expected_status {
        401 => {
            // RFC 7235: 401 must have WWW-Authenticate
            let www_auth = headers.get("WWW-Authenticate");
            assert!(
                www_auth.is_some(),
                "401 responses must have WWW-Authenticate header per RFC 7235"
            );
            assert_eq!(www_auth.unwrap().to_str().unwrap(), "Bearer");
            // Verify no Retry-After on 401
            assert!(
                headers.get("Retry-After").is_none(),
                "401 responses must not have Retry-After header"
            );
        }
        503 => {
            // RFC 7231: 503 should have Retry-After
            let retry_after = headers.get("Retry-After");
            assert!(
                retry_after.is_some(),
                "503 responses must have Retry-After header per RFC 7231"
            );
            assert!(
                !retry_after.unwrap().to_str().unwrap().is_empty(),
                "Retry-After should not be empty"
            );
            // Verify no WWW-Authenticate on 503
            assert!(
                headers.get("WWW-Authenticate").is_none(),
                "503 responses must not have WWW-Authenticate header"
            );
        }
        400 | 404 | 409 => {
            // These status codes should have neither special header
            assert!(
                headers.get("WWW-Authenticate").is_none(),
                "{expected_status} responses must not have WWW-Authenticate header"
            );
            assert!(
                headers.get("Retry-After").is_none(),
                "{expected_status} responses must not have Retry-After header"
            );
        }
        _ => {
            // For other status codes, no specific header requirements are enforced
        }
    }

    // Read and parse the response body
    let body = test::read_body(resp).await;
    let body_str = std::str::from_utf8(&body).expect("Response body should be valid UTF-8");

    // Improved error handling for deserialization failures with more descriptive error message
    let problem_details: Value = serde_json::from_str(body_str).unwrap_or_else(|_| {
        panic!("Failed to parse error body as ProblemDetails. Raw body: {body_str}")
    });

    // Assert all required keys are present
    for key in ["type", "title", "status", "detail", "code", "trace_id"] {
        assert!(
            problem_details.get(key).is_some(),
            "{key} field should be present"
        );
    }

    // Assert specific values
    assert_eq!(problem_details["code"], expected_code);
    assert_eq!(problem_details["detail"], expected_detail);
    assert_eq!(problem_details["status"], expected_status);

    // Use centralized trace_id validation
    assert_trace_id_matches(&problem_details, trace_id);

    // Assert type follows the expected format
    let type_value = problem_details["type"]
        .as_str()
        .expect("type field should be a string");
    assert!(
        type_value.starts_with("https://nommie.app/errors/"),
        "type should follow the expected URL format"
    );
}

/// Test-only helper to run a closure within a savepoint that gets rolled back.
/// This allows testing database constraint violations without poisoning the outer transaction.
pub async fn with_savepoint<F, Fut, T>(
    outer: &(impl sea_orm::ConnectionTrait + sea_orm::TransactionTrait),
    f: F,
) -> Result<T, backend::error::AppError>
where
    F: FnOnce(sea_orm::DatabaseTransaction) -> Fut,
    Fut: std::future::Future<Output = Result<T, backend::error::AppError>>,
{
    let sp = outer
        .begin()
        .await
        .map_err(|e| backend::error::AppError::from(backend::infra::db_errors::map_db_err(e)))?;
    let out = f(sp).await;
    // Note: The transaction will be automatically rolled back when dropped
    out
}
