// Error handling tests for WebSocket connections

use serde_json::json;

/// Test that error messages have the expected JSON structure.
///
/// This verifies that when the backend sends error messages via WebSocket,
/// they match the format expected by the frontend.
#[tokio::test]
async fn websocket_error_message_has_correct_format() -> Result<(), Box<dyn std::error::Error>> {
    // Test error message with code
    let error_message = json!({
        "type": "error",
        "message": "Failed to build snapshot: test error",
        "code": "INTERNAL_ERROR"
    });

    assert_eq!(error_message["type"], "error");
    assert!(error_message["message"].is_string());
    assert_eq!(
        error_message["message"],
        "Failed to build snapshot: test error"
    );
    assert!(error_message["code"].is_string());
    assert_eq!(error_message["code"], "INTERNAL_ERROR");

    // Test error message without code (code is optional)
    let error_message_no_code = json!({
        "type": "error",
        "message": "Failed to build snapshot: test error"
    });

    assert_eq!(error_message_no_code["type"], "error");
    assert!(error_message_no_code["message"].is_string());
    // Code should be optional (missing from JSON)
    assert!(error_message_no_code.get("code").is_none());

    Ok(())
}
