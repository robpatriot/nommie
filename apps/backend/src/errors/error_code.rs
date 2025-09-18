//! Error codes for the Nommie backend API.
//!
//! This module defines all error codes used throughout the application.
//! Add new codes here; never pass ad-hoc strings as error codes.
//!
//! All error codes are SCREAMING_SNAKE_CASE and map 1:1 to the strings
//! that appear in HTTP responses.

use core::fmt;

/// Centralized error codes for the Nommie backend API.
///
/// This enum ensures type safety and prevents the use of ad-hoc error codes.
/// Each variant maps to a canonical SCREAMING_SNAKE_CASE string that appears
/// in HTTP responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // Authentication & Authorization
    /// Authentication required
    Unauthorized,
    /// Missing or malformed Bearer token
    UnauthorizedMissingBearer,
    /// Invalid JWT token
    UnauthorizedInvalidJwt,
    /// JWT token has expired
    UnauthorizedExpiredJwt,
    /// Access denied
    Forbidden,
    /// User not found in database
    ForbiddenUserNotFound,

    // Request Validation
    /// Invalid game ID provided
    InvalidGameId,
    /// Invalid email address
    InvalidEmail,
    /// Invalid Google sub provided
    InvalidGoogleSub,
    /// General validation error
    ValidationError,
    /// General bad request error
    BadRequest,

    // Resource Not Found
    /// Game not found
    GameNotFound,
    /// General not found error
    NotFound,

    // Business Logic Conflicts
    /// Google sub mismatch for existing email
    GoogleSubMismatch,

    // System Errors
    /// Database error
    DbError,
    /// Database unavailable
    DbUnavailable,
    /// Internal server error
    Internal,
    /// Configuration error
    ConfigError,

    // Test-only codes
    /// Test error without trace
    NoTrace,
}

impl ErrorCode {
    /// Returns the canonical SCREAMING_SNAKE_CASE string for this error code.
    ///
    /// This is the exact string that appears in HTTP responses.
    pub const fn as_str(&self) -> &'static str {
        match self {
            // Authentication & Authorization
            Self::Unauthorized => "UNAUTHORIZED",
            Self::UnauthorizedMissingBearer => "UNAUTHORIZED_MISSING_BEARER",
            Self::UnauthorizedInvalidJwt => "UNAUTHORIZED_INVALID_JWT",
            Self::UnauthorizedExpiredJwt => "UNAUTHORIZED_EXPIRED_JWT",
            Self::Forbidden => "FORBIDDEN",
            Self::ForbiddenUserNotFound => "FORBIDDEN_USER_NOT_FOUND",

            // Request Validation
            Self::InvalidGameId => "INVALID_GAME_ID",
            Self::InvalidEmail => "INVALID_EMAIL",
            Self::InvalidGoogleSub => "INVALID_GOOGLE_SUB",
            Self::ValidationError => "VALIDATION_ERROR",
            Self::BadRequest => "BAD_REQUEST",

            // Resource Not Found
            Self::GameNotFound => "GAME_NOT_FOUND",
            Self::NotFound => "NOT_FOUND",

            // Business Logic Conflicts
            Self::GoogleSubMismatch => "GOOGLE_SUB_MISMATCH",

            // System Errors
            Self::DbError => "DB_ERROR",
            Self::DbUnavailable => "DB_UNAVAILABLE",
            Self::Internal => "INTERNAL",
            Self::ConfigError => "CONFIG_ERROR",

            // Test-only codes
            Self::NoTrace => "NO_TRACE",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_strings() {
        // Verify that all error codes produce the expected SCREAMING_SNAKE_CASE strings
        assert_eq!(ErrorCode::Unauthorized.as_str(), "UNAUTHORIZED");
        assert_eq!(
            ErrorCode::UnauthorizedMissingBearer.as_str(),
            "UNAUTHORIZED_MISSING_BEARER"
        );
        assert_eq!(
            ErrorCode::UnauthorizedInvalidJwt.as_str(),
            "UNAUTHORIZED_INVALID_JWT"
        );
        assert_eq!(
            ErrorCode::UnauthorizedExpiredJwt.as_str(),
            "UNAUTHORIZED_EXPIRED_JWT"
        );
        assert_eq!(ErrorCode::Forbidden.as_str(), "FORBIDDEN");
        assert_eq!(
            ErrorCode::ForbiddenUserNotFound.as_str(),
            "FORBIDDEN_USER_NOT_FOUND"
        );
        assert_eq!(ErrorCode::InvalidGameId.as_str(), "INVALID_GAME_ID");
        assert_eq!(ErrorCode::InvalidEmail.as_str(), "INVALID_EMAIL");
        assert_eq!(ErrorCode::InvalidGoogleSub.as_str(), "INVALID_GOOGLE_SUB");
        assert_eq!(ErrorCode::ValidationError.as_str(), "VALIDATION_ERROR");
        assert_eq!(ErrorCode::BadRequest.as_str(), "BAD_REQUEST");
        assert_eq!(ErrorCode::GameNotFound.as_str(), "GAME_NOT_FOUND");
        assert_eq!(ErrorCode::NotFound.as_str(), "NOT_FOUND");
        assert_eq!(ErrorCode::GoogleSubMismatch.as_str(), "GOOGLE_SUB_MISMATCH");
        assert_eq!(ErrorCode::DbError.as_str(), "DB_ERROR");
        assert_eq!(ErrorCode::DbUnavailable.as_str(), "DB_UNAVAILABLE");
        assert_eq!(ErrorCode::Internal.as_str(), "INTERNAL");
        assert_eq!(ErrorCode::ConfigError.as_str(), "CONFIG_ERROR");
        assert_eq!(ErrorCode::NoTrace.as_str(), "NO_TRACE");
    }

    #[test]
    fn test_display_trait() {
        assert_eq!(format!("{}", ErrorCode::Unauthorized), "UNAUTHORIZED");
        assert_eq!(format!("{}", ErrorCode::InvalidGameId), "INVALID_GAME_ID");
    }
}
