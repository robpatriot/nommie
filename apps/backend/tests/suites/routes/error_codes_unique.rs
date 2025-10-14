use std::collections::HashSet;

use backend::errors::ErrorCode;

#[test]
fn error_codes_are_unique() {
    let all = [
        // Keep in sync with ErrorCode enum variants
        ErrorCode::Unauthorized,
        ErrorCode::UnauthorizedMissingBearer,
        ErrorCode::UnauthorizedInvalidJwt,
        ErrorCode::UnauthorizedExpiredJwt,
        ErrorCode::Forbidden,
        ErrorCode::ForbiddenUserNotFound,
        ErrorCode::NotAMember,
        ErrorCode::InsufficientRole,
        ErrorCode::InvalidGameId,
        ErrorCode::InvalidEmail,
        ErrorCode::InvalidGoogleSub,
        ErrorCode::ValidationError,
        ErrorCode::BadRequest,
        ErrorCode::GameNotFound,
        ErrorCode::UserNotFound,
        ErrorCode::NotFound,
        ErrorCode::GoogleSubMismatch,
        ErrorCode::JoinCodeConflict,
        ErrorCode::SeatTaken,
        ErrorCode::UniqueEmail,
        ErrorCode::OptimisticLock,
        ErrorCode::Conflict,
        ErrorCode::DbError,
        ErrorCode::DbUnavailable,
        ErrorCode::DbPoolExhausted,
        ErrorCode::DbTimeout,
        ErrorCode::UniqueViolation,
        ErrorCode::FkViolation,
        ErrorCode::CheckViolation,
        ErrorCode::RecordNotFound,
        ErrorCode::Internal,
        ErrorCode::InternalError,
        ErrorCode::ConfigError,
        ErrorCode::DataCorruption,
        ErrorCode::NoTrace,
    ];

    let mut seen = HashSet::new();
    for code in all {
        let s = code.as_str();
        assert!(seen.insert(s), "Duplicate error code string: {s}");
    }
}
