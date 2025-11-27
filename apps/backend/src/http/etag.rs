//! ETag helpers for optimistic concurrency control.
//!
//! This module provides utilities for generating and parsing ETags for games,
//! enabling HTTP-native optimistic locking via ETag/If-Match headers.

use actix_web::dev::Payload;
use actix_web::http::header::IF_MATCH;
use actix_web::{FromRequest, HttpRequest};

use crate::error::AppError;
use crate::errors::ErrorCode;

/// Generate an ETag for a game resource.
///
/// Format: `"game-{id}-v{version}"` (with quotes, as required by HTTP spec)
///
/// # Example
/// ```
/// # use backend::http::etag::game_etag;
/// let etag = game_etag(123, 5);
/// assert_eq!(etag, r#""game-123-v5""#);
/// ```
pub fn game_etag(id: i64, version: i32) -> String {
    format!(r#""game-{id}-v{version}""#)
}

/// Parse the lock version from a game ETag value.
///
/// Accepts ETags in the format `"game-{id}-v{version}"` and extracts the version number.
///
/// # Errors
/// Returns `AppError::bad_request` with `ErrorCode::InvalidHeader` if:
/// - The ETag is missing or malformed
/// - The version cannot be parsed as i32
///
/// # Example
/// ```
/// # use backend::http::etag::parse_game_version_from_etag;
/// let version = parse_game_version_from_etag(r#""game-123-v5""#).unwrap();
/// assert_eq!(version, 5);
/// ```
pub fn parse_game_version_from_etag(s: &str) -> Result<i32, AppError> {
    // Remove quotes if present and trim whitespace
    let s = s.trim_matches('"').trim();

    // Expected format: game-{id}-v{version}
    // We need to extract the version after the last "-v"
    let version_prefix = "-v";
    let version_start = s
        .rfind(version_prefix)
        .ok_or_else(|| {
            AppError::bad_request(
                ErrorCode::InvalidHeader,
                format!("Invalid ETag format: missing version marker. Expected format: \"game-{{id}}-v{{version}}\", got: \"{s}\""),
            )
        })?
        + version_prefix.len();

    // Extract only the numeric part (stop at first non-digit character)
    // This prevents issues where additional text might be appended (e.g., "11-gzip")
    let version_slice = &s[version_start..];
    let version_end = version_slice
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(idx, _)| idx)
        .unwrap_or(version_slice.len());

    let version_str = &version_slice[..version_end];

    if version_str.is_empty() {
        return Err(AppError::bad_request(
            ErrorCode::InvalidHeader,
            format!("Invalid ETag format: version is empty. Expected format: \"game-{{id}}-v{{version}}\", got: \"{s}\""),
        ));
    }

    version_str.parse::<i32>().map_err(|_| {
        AppError::bad_request(
            ErrorCode::InvalidHeader,
            format!("Invalid ETag format: version must be a valid integer, got: \"{version_str}\" (from ETag: \"{s}\")"),
        )
    })
}

/// Extractor that reads and parses the `If-Match` header to obtain the expected lock version.
///
/// This extractor is useful for PATCH and DELETE handlers that need to enforce optimistic locking.
///
/// # Errors
/// Returns `AppError::precondition_required` (HTTP 428) if the `If-Match` header is missing.
/// Returns `AppError::bad_request` with `ErrorCode::InvalidHeader` (HTTP 400) if:
/// - The `If-Match` header cannot be parsed as a valid game ETag
/// - The version cannot be extracted from the ETag
///
/// # Example
/// ```ignore
/// async fn update_game(
///     game_id: GameId,
///     expected_version: ExpectedVersion,
/// ) -> Result<HttpResponse, AppError> {
///     // Use expected_version.0 as the lock version
///     Ok(HttpResponse::Ok().finish())
/// }
/// ```
pub struct ExpectedVersion(pub i32);

impl FromRequest for ExpectedVersion {
    type Error = AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // Extract If-Match header - return 428 if missing
            let if_match = req.headers().get(IF_MATCH).ok_or_else(|| {
                AppError::precondition_required("If-Match header is required for this operation")
            })?;

            let if_match_str = if_match.to_str().map_err(|_| {
                AppError::bad_request(
                    ErrorCode::InvalidHeader,
                    "If-Match header contains invalid characters",
                )
            })?;

            // RFC 7232: If-Match can contain multiple ETags separated by commas
            // We take the first valid ETag. Split by comma and trim whitespace.
            let etag_candidates: Vec<&str> = if_match_str.split(',').map(|s| s.trim()).collect();

            // Try to parse each ETag candidate until we find a valid one
            let mut last_error = None;
            for candidate in etag_candidates {
                // Skip wildcard
                if candidate == "*" {
                    continue;
                }

                match parse_game_version_from_etag(candidate) {
                    Ok(version) => return Ok(ExpectedVersion(version)),
                    Err(e) => {
                        last_error = Some((candidate.to_string(), e));
                    }
                }
            }

            // If we got here, none of the ETags were valid
            let (failed_etag, error) = last_error.unwrap_or_else(|| {
                (
                    if_match_str.to_string(),
                    AppError::bad_request(
                        ErrorCode::InvalidHeader,
                        "If-Match header contains no valid ETags",
                    ),
                )
            });

            Err(AppError::bad_request(
                ErrorCode::InvalidHeader,
                format!(
                    "Invalid If-Match header: failed to parse ETag \"{}\" from header value \"{}\". Original error: {}",
                    failed_etag,
                    if_match_str,
                    error
                ),
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_etag_format() {
        assert_eq!(game_etag(123, 5), r#""game-123-v5""#);
        assert_eq!(game_etag(1, 0), r#""game-1-v0""#);
        assert_eq!(game_etag(999999, 42), r#""game-999999-v42""#);
    }

    #[test]
    fn test_parse_game_version_from_etag_success() {
        assert_eq!(parse_game_version_from_etag(r#""game-123-v5""#).unwrap(), 5);
        assert_eq!(parse_game_version_from_etag(r#""game-1-v0""#).unwrap(), 0);
        assert_eq!(
            parse_game_version_from_etag(r#""game-999-v42""#).unwrap(),
            42
        );

        // Should work without quotes too
        assert_eq!(parse_game_version_from_etag("game-123-v5").unwrap(), 5);
    }

    #[test]
    fn test_parse_game_version_from_etag_invalid_format() {
        let result = parse_game_version_from_etag("invalid");
        assert!(result.is_err());

        let result = parse_game_version_from_etag(r#""game-123""#);
        assert!(result.is_err());

        let result = parse_game_version_from_etag(r#""wrongformat""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_game_version_from_etag_invalid_version() {
        let result = parse_game_version_from_etag(r#""game-123-vabc""#);
        assert!(result.is_err());

        let result = parse_game_version_from_etag(r#""game-123-v""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_game_version_from_etag_with_suffix() {
        // Test that parsing stops at first non-digit character
        // This handles cases where additional text might be appended (e.g., "11-gzip")
        let result = parse_game_version_from_etag(r#""game-123-v11-gzip""#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 11);

        let result = parse_game_version_from_etag("game-123-v42-extra");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_roundtrip() {
        let id = 123;
        let version = 5;
        let etag = game_etag(id, version);
        let parsed_version = parse_game_version_from_etag(&etag).unwrap();
        assert_eq!(version, parsed_version);
    }
}
