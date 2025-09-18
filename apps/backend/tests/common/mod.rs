use actix_web::test;
use serde_json::Value;

/// Helper function to check that the trace_id in the response body matches the X-Trace-Id header
pub fn assert_trace_id_matches(response: &Value, header_trace_id: &str) {
    let trace_id_in_body = response["trace_id"].as_str().unwrap();
    assert_eq!(
        trace_id_in_body, header_trace_id,
        "trace_id in body should match X-Trace-Id header"
    );
}

/// Helper function to validate that a response follows the ProblemDetails structure
/// and that trace_id matches the X-Trace-Id header
pub async fn assert_problem_details_structure(
    resp: actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
    expected_status: u16,
    expected_code: &str,
    expected_detail: &str,
) {
    // Assert status code
    assert_eq!(resp.status().as_u16(), expected_status);

    // Extract headers before consuming the response
    let headers = resp.headers().clone();
    let trace_id_header = headers.get("x-trace-id");
    assert!(
        trace_id_header.is_some(),
        "X-Trace-Id header should be present"
    );
    let trace_id = trace_id_header.unwrap().to_str().unwrap();
    assert!(
        !trace_id.is_empty(),
        "X-Trace-Id header should not be empty"
    );

    // Assert Content-Type is application/problem+json
    let content_type = headers.get("content-type").unwrap().to_str().unwrap();
    assert_eq!(content_type, "application/problem+json");

    // Read and parse the response body
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Improved error handling for deserialization failures with more descriptive error message
    let problem_details: Value = match serde_json::from_str(&body_str) {
        Ok(details) => details,
        Err(_) => panic!("Failed to parse error body as ProblemDetails. Raw body: {body_str}"),
    };

    // Assert all required keys are present
    assert!(
        problem_details.get("type").is_some(),
        "type field should be present"
    );
    assert!(
        problem_details.get("title").is_some(),
        "title field should be present"
    );
    assert!(
        problem_details.get("status").is_some(),
        "status field should be present"
    );
    assert!(
        problem_details.get("detail").is_some(),
        "detail field should be present"
    );
    assert!(
        problem_details.get("code").is_some(),
        "code field should be present"
    );
    assert!(
        problem_details.get("trace_id").is_some(),
        "trace_id field should be present"
    );

    // Assert specific values
    assert_eq!(problem_details["code"], expected_code);
    assert_eq!(problem_details["detail"], expected_detail);
    assert_eq!(problem_details["status"], expected_status);

    // Use centralized trace_id validation
    assert_trace_id_matches(&problem_details, trace_id);

    // Assert type follows the expected format
    let type_value = problem_details["type"].as_str().unwrap();
    assert!(
        type_value.starts_with("https://nommie.app/errors/"),
        "type should follow the expected URL format"
    );
}
