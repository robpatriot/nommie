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
    /// User is not a member of the game
    NotAMember,
    /// User has insufficient role for this operation
    InsufficientRole,

    // Request Validation
    /// Invalid game ID provided
    InvalidGameId,
    /// Invalid email address
    InvalidEmail,
    /// Invalid Google sub provided
    InvalidGoogleSub,
    /// Invalid seat number
    InvalidSeat,
    /// Invalid bid provided
    InvalidBid,
    /// Must follow suit
    MustFollowSuit,
    /// Card not in hand
    CardNotInHand,
    /// Out of turn
    OutOfTurn,
    /// Phase mismatch
    PhaseMismatch,
    /// Parse card error
    ParseCard,
    /// Invalid trump conversion
    InvalidTrumpConversion,
    /// General validation error
    ValidationError,
    /// General bad request error
    BadRequest,
    /// Invalid or missing HTTP header
    InvalidHeader,
    /// Precondition required for this operation
    PreconditionRequired,

    // Resource Not Found
    /// Game not found
    GameNotFound,
    /// User not found
    UserNotFound,
    /// Player not found
    PlayerNotFound,
    /// General not found error
    NotFound,

    // Business Logic Conflicts
    /// Google sub mismatch for existing email
    GoogleSubMismatch,
    /// Join code already exists
    JoinCodeConflict,
    /// Seat already taken
    SeatTaken,
    /// Unique email constraint
    UniqueEmail,
    /// Optimistic lock conflict
    OptimisticLock,
    /// Generic conflict (fallback for unmatched conflicts)
    Conflict,

    // System Errors
    /// Database error
    DbError,
    /// Database unavailable
    DbUnavailable,
    /// Database pool exhausted
    DbPoolExhausted,
    /// Database timeout (gateway timeout)
    DbTimeout,

    // Database Constraint Violations
    /// Unique constraint violation (SQLSTATE 23505; generic 409)
    UniqueViolation,
    /// Foreign key constraint violation (SQLSTATE 23503; generic 409)
    FkViolation,
    /// Check constraint violation (SQLSTATE 23514; generic 400)
    CheckViolation,
    /// Record not found (generic 404 for DB-driven not-found)
    RecordNotFound,

    /// Internal server error
    Internal,
    /// Internal server error (explicit problem code)
    InternalError,
    /// Configuration error
    ConfigError,
    /// Data corruption detected
    DataCorruption,

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
            Self::NotAMember => "NOT_A_MEMBER",
            Self::InsufficientRole => "INSUFFICIENT_ROLE",

            // Request Validation
            Self::InvalidGameId => "INVALID_GAME_ID",
            Self::InvalidEmail => "INVALID_EMAIL",
            Self::InvalidGoogleSub => "INVALID_GOOGLE_SUB",
            Self::InvalidSeat => "INVALID_SEAT",
            Self::InvalidBid => "INVALID_BID",
            Self::MustFollowSuit => "MUST_FOLLOW_SUIT",
            Self::CardNotInHand => "CARD_NOT_IN_HAND",
            Self::OutOfTurn => "OUT_OF_TURN",
            Self::PhaseMismatch => "PHASE_MISMATCH",
            Self::ParseCard => "PARSE_CARD",
            Self::InvalidTrumpConversion => "INVALID_TRUMP_CONVERSION",
            Self::ValidationError => "VALIDATION_ERROR",
            Self::BadRequest => "BAD_REQUEST",
            Self::InvalidHeader => "INVALID_HEADER",
            Self::PreconditionRequired => "PRECONDITION_REQUIRED",

            // Resource Not Found
            Self::GameNotFound => "GAME_NOT_FOUND",
            Self::UserNotFound => "USER_NOT_FOUND",
            Self::PlayerNotFound => "PLAYER_NOT_FOUND",
            Self::NotFound => "NOT_FOUND",

            // Business Logic Conflicts
            Self::GoogleSubMismatch => "GOOGLE_SUB_MISMATCH",
            Self::JoinCodeConflict => "JOIN_CODE_CONFLICT",
            Self::SeatTaken => "SEAT_TAKEN",
            Self::UniqueEmail => "UNIQUE_EMAIL",
            Self::OptimisticLock => "OPTIMISTIC_LOCK",
            Self::Conflict => "CONFLICT",

            // System Errors
            Self::DbError => "DB_ERROR",
            Self::DbUnavailable => "DB_UNAVAILABLE",
            Self::DbPoolExhausted => "DB_POOL_EXHAUSTED",
            Self::DbTimeout => "DB_TIMEOUT",

            // Database Constraint Violations
            Self::UniqueViolation => "UNIQUE_VIOLATION",
            Self::FkViolation => "FK_VIOLATION",
            Self::CheckViolation => "CHECK_VIOLATION",
            Self::RecordNotFound => "RECORD_NOT_FOUND",

            Self::Internal => "INTERNAL",
            Self::InternalError => "INTERNAL_ERROR",
            Self::ConfigError => "CONFIG_ERROR",
            Self::DataCorruption => "DATA_CORRUPTION",

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
        assert_eq!(ErrorCode::NotAMember.as_str(), "NOT_A_MEMBER");
        assert_eq!(ErrorCode::InsufficientRole.as_str(), "INSUFFICIENT_ROLE");
        assert_eq!(ErrorCode::InvalidGameId.as_str(), "INVALID_GAME_ID");
        assert_eq!(ErrorCode::InvalidEmail.as_str(), "INVALID_EMAIL");
        assert_eq!(ErrorCode::InvalidGoogleSub.as_str(), "INVALID_GOOGLE_SUB");
        assert_eq!(ErrorCode::InvalidSeat.as_str(), "INVALID_SEAT");
        assert_eq!(ErrorCode::InvalidBid.as_str(), "INVALID_BID");
        assert_eq!(ErrorCode::MustFollowSuit.as_str(), "MUST_FOLLOW_SUIT");
        assert_eq!(ErrorCode::CardNotInHand.as_str(), "CARD_NOT_IN_HAND");
        assert_eq!(ErrorCode::OutOfTurn.as_str(), "OUT_OF_TURN");
        assert_eq!(ErrorCode::PhaseMismatch.as_str(), "PHASE_MISMATCH");
        assert_eq!(ErrorCode::ParseCard.as_str(), "PARSE_CARD");
        assert_eq!(
            ErrorCode::InvalidTrumpConversion.as_str(),
            "INVALID_TRUMP_CONVERSION"
        );
        assert_eq!(ErrorCode::ValidationError.as_str(), "VALIDATION_ERROR");
        assert_eq!(ErrorCode::BadRequest.as_str(), "BAD_REQUEST");
        assert_eq!(ErrorCode::InvalidHeader.as_str(), "INVALID_HEADER");
        assert_eq!(
            ErrorCode::PreconditionRequired.as_str(),
            "PRECONDITION_REQUIRED"
        );
        assert_eq!(ErrorCode::GameNotFound.as_str(), "GAME_NOT_FOUND");
        assert_eq!(ErrorCode::PlayerNotFound.as_str(), "PLAYER_NOT_FOUND");
        assert_eq!(ErrorCode::NotFound.as_str(), "NOT_FOUND");
        assert_eq!(ErrorCode::GoogleSubMismatch.as_str(), "GOOGLE_SUB_MISMATCH");
        assert_eq!(ErrorCode::JoinCodeConflict.as_str(), "JOIN_CODE_CONFLICT");
        assert_eq!(ErrorCode::DbError.as_str(), "DB_ERROR");
        assert_eq!(ErrorCode::DbUnavailable.as_str(), "DB_UNAVAILABLE");
        assert_eq!(ErrorCode::DbPoolExhausted.as_str(), "DB_POOL_EXHAUSTED");
        assert_eq!(ErrorCode::UniqueViolation.as_str(), "UNIQUE_VIOLATION");
        assert_eq!(ErrorCode::FkViolation.as_str(), "FK_VIOLATION");
        assert_eq!(ErrorCode::CheckViolation.as_str(), "CHECK_VIOLATION");
        assert_eq!(ErrorCode::RecordNotFound.as_str(), "RECORD_NOT_FOUND");
        assert_eq!(ErrorCode::Internal.as_str(), "INTERNAL");
        assert_eq!(ErrorCode::ConfigError.as_str(), "CONFIG_ERROR");
        assert_eq!(ErrorCode::NoTrace.as_str(), "NO_TRACE");
    }

    #[test]
    fn test_display_trait() {
        assert_eq!(format!("{}", ErrorCode::Unauthorized), "UNAUTHORIZED");
        assert_eq!(format!("{}", ErrorCode::InvalidGameId), "INVALID_GAME_ID");
        assert_eq!(
            format!("{}", ErrorCode::UniqueViolation),
            "UNIQUE_VIOLATION"
        );
        assert_eq!(format!("{}", ErrorCode::FkViolation), "FK_VIOLATION");
        assert_eq!(format!("{}", ErrorCode::CheckViolation), "CHECK_VIOLATION");
        assert_eq!(format!("{}", ErrorCode::RecordNotFound), "RECORD_NOT_FOUND");
        assert_eq!(
            format!("{}", ErrorCode::DbPoolExhausted),
            "DB_POOL_EXHAUSTED"
        );
    }
}
