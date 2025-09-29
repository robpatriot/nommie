use std::ops::{Deref, DerefMut};

use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use bytes::BytesMut;
use futures_util::StreamExt;
use serde::de::DeserializeOwned;
use serde_json::Error as JsonError;
use tracing::{debug, warn};

use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::logging::pii::Redacted;
use crate::web::trace_ctx;

/// Validated JSON extractor that provides standardized error handling for JSON parse/validation failures
///
/// This extractor deserializes request bodies and converts any JSON parse/validation failures
/// into our standardized AppError (RFC7807 with trace_id) using HTTP 400 and the project's
/// canonical "bad request / validation" error code.
#[derive(Debug)]
pub struct ValidatedJson<T>(pub T);

impl<T> ValidatedJson<T> {
    /// Extract the inner value from the ValidatedJson wrapper
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ValidatedJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> FromRequest for ValidatedJson<T>
where
    T: DeserializeOwned + 'static,
{
    type Error = AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();

        // Extract content type before creating the async future to avoid borrow-across-await
        let content_type = req
            .headers()
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("")
            .to_string();

        Box::pin(async move {
            let trace_id = trace_ctx::trace_id();

            // Collect the request body into BytesMut
            let mut body = BytesMut::new();
            while let Some(chunk) = payload.next().await {
                let chunk = chunk.map_err(|e| {
                    warn!(
                        trace_id = %trace_id,
                        error = %e,
                        "Failed to read request body chunk"
                    );
                    AppError::bad_request(
                        ErrorCode::BadRequest,
                        "Failed to read request body".to_string(),
                    )
                })?;
                body.extend_from_slice(&chunk);
            }

            // Attempt to parse JSON
            let parsed = serde_json::from_slice::<T>(&body).map_err(|e| {
                let detail = classify_json_error(&e);

                debug!(
                    trace_id = %trace_id,
                    error = %Redacted(&e.to_string()),
                    content_type = %content_type,
                    body_size = body.len(),
                    "JSON parsing failed"
                );

                AppError::bad_request(ErrorCode::BadRequest, detail)
            })?;

            Ok(ValidatedJson(parsed))
        })
    }
}

/// Classify serde_json::Error and return a sanitized error message
fn classify_json_error(error: &JsonError) -> String {
    match error.classify() {
        serde_json::error::Category::Syntax => {
            let line = error.line();
            format!("Invalid JSON at line {line}")
        }
        serde_json::error::Category::Eof => "Invalid JSON: unexpected end of input".to_string(),
        serde_json::error::Category::Data => {
            "Invalid JSON: wrong types for one or more fields".to_string()
        }
        serde_json::error::Category::Io => "Invalid JSON: I/O error while reading body".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Deserialize, Serialize)]
    struct TestStruct {
        pub name: String,
        pub age: u32,
    }

    #[test]
    fn test_classify_json_error_syntax() {
        let json = r#"{"name": "test", "age": }"#;
        let error = serde_json::from_str::<TestStruct>(json).unwrap_err();
        let detail = classify_json_error(&error);
        assert!(detail.contains("Invalid JSON"));
        assert!(detail.contains("line") || detail.contains("syntax"));
    }

    #[test]
    fn test_classify_json_error_eof() {
        let json = r#"{"name": "test""#;
        let error = serde_json::from_str::<TestStruct>(json).unwrap_err();
        let detail = classify_json_error(&error);
        assert!(detail.contains("unexpected end of input"));
    }

    #[test]
    fn test_classify_json_error_data() {
        let json = r#"{"name": 123, "age": "invalid"}"#;
        let error = serde_json::from_str::<TestStruct>(json).unwrap_err();
        let detail = classify_json_error(&error);
        assert!(detail.contains("wrong types"));
    }

    #[test]
    fn test_validated_json_deref() {
        let data = TestStruct {
            name: "test".to_string(),
            age: 42,
        };
        let validated = ValidatedJson(data);

        assert_eq!(validated.name, "test");
        assert_eq!(validated.age, 42);
    }

    #[test]
    fn test_validated_json_deref_mut() {
        let data = TestStruct {
            name: "test".to_string(),
            age: 42,
        };
        let mut validated = ValidatedJson(data);

        validated.name = "updated".to_string();
        validated.age = 24;

        assert_eq!(validated.name, "updated");
        assert_eq!(validated.age, 24);
    }

    #[test]
    fn test_validated_json_into_inner() {
        let data = TestStruct {
            name: "test".to_string(),
            age: 42,
        };
        let validated = ValidatedJson(data);
        let inner = validated.into_inner();

        assert_eq!(inner.name, "test");
        assert_eq!(inner.age, 42);
    }
}
