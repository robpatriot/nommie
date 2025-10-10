//! ETag helpers for optimistic concurrency control.
//!
//! This module provides utilities for generating and parsing ETags for games,
//! enabling HTTP-native optimistic locking via ETag/If-Match headers.

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
    // Remove quotes if present
    let s = s.trim_matches('"');

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

    let version_str = &s[version_start..];
    version_str.parse::<i32>().map_err(|_| {
        AppError::bad_request(
            ErrorCode::InvalidHeader,
            format!("Invalid ETag format: version must be a valid integer, got: \"{version_str}\""),
        )
    })
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
    fn test_roundtrip() {
        let id = 123;
        let version = 5;
        let etag = game_etag(id, version);
        let parsed_version = parse_game_version_from_etag(&etag).unwrap();
        assert_eq!(version, parsed_version);
    }
}
