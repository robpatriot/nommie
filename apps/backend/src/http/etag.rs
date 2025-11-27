//! ETag helpers for HTTP cache validation.
//!
//! This module provides utilities for generating ETags for games,
//! used for HTTP conditional GET requests (If-None-Match).

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_etag_format() {
        assert_eq!(game_etag(123, 5), r#""game-123-v5""#);
        assert_eq!(game_etag(1, 0), r#""game-1-v0""#);
        assert_eq!(game_etag(999999, 42), r#""game-999999-v42""#);
    }
}
