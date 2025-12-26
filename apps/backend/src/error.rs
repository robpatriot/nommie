//! Error handling for the Nommie backend.
//!
//! This module provides comprehensive error handling with precise database error mapping.
//! Database errors are mapped using structured variants where possible, falling back to
//! SQLSTATE code inspection for constraint violations:
//! - 23505: Unique constraint violation → Conflict (409)
//! - 23503: Foreign key constraint violation → Conflict (409)  
//! - 23514: Check constraint violation → Bad Request (400)
//!
//! All errors follow RFC 7807 Problem Details format with proper HTTP status codes.

use actix_web::error::ResponseError;
use actix_web::http::header::{CONTENT_TYPE, RETRY_AFTER, WWW_AUTHENTICATE};
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use serde::Serialize;
use thiserror::Error;
use tracing::{error, warn};

use crate::errors::ErrorCode;
// use crate::logging::pii::Redacted; // not used in this module
use crate::trace_ctx;

/// Boxed error type for storage in AppError variants
type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Sentinel error type for cases where no real underlying error exists
#[derive(Debug, Error)]
#[error("{0}")]
pub struct Sentinel(pub &'static str);

/// Helper to convert DomainError detail strings into error sources
#[derive(Debug)]
struct DomainErrorWrapper(String);

impl std::error::Error for DomainErrorWrapper {}

impl std::fmt::Display for DomainErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Classify transient database errors into a suggested Retry-After (in seconds).
///
/// Rules:
/// - SQLite contention: message contains "database is locked" → Some(1)
/// - Postgres transients by SQLSTATE code:
///   - codes starting with "08" (connection exceptions) → Some(1)
///   - codes equal to one of: "55P03", "40001", "40P01", "53300" → Some(1)
/// - Otherwise → None
pub fn classify_transient(code: Option<&str>, message: &str) -> Option<u32> {
    if message.contains("database is locked") {
        return Some(1);
    }
    if let Some(c) = code {
        if c.starts_with("08") {
            return Some(1);
        }
        if matches!(c, "55P03" | "40001" | "40P01" | "53300") {
            return Some(1);
        }
    }
    None
}

#[derive(Serialize)]
pub struct ProblemDetails {
    #[serde(rename = "type")]
    pub type_: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    pub code: String,
    pub trace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Validation error: {detail}")]
    Validation {
        code: ErrorCode,
        detail: String,
        status: StatusCode,
    },
    #[error("Database error: {detail}")]
    Db {
        detail: String,
        #[source]
        source: BoxError,
    },
    #[error("Not found: {detail}")]
    NotFound { code: ErrorCode, detail: String },
    #[error("Unauthorized")]
    Unauthorized,
    #[error("UnauthorizedMissingBearer")]
    UnauthorizedMissingBearer,
    #[error("UnauthorizedInvalidJwt")]
    UnauthorizedInvalidJwt,
    #[error("UnauthorizedExpiredJwt")]
    UnauthorizedExpiredJwt,
    #[error("Forbidden: {detail}")]
    Forbidden { code: ErrorCode, detail: String },
    #[error("Forbidden: User not found")]
    ForbiddenUserNotFound,
    #[error("Bad request: {detail}")]
    BadRequest { code: ErrorCode, detail: String },
    #[error("Internal error: {detail}")]
    Internal {
        code: ErrorCode,
        detail: String,
        #[source]
        source: BoxError,
    },
    #[error("Configuration error: {detail}")]
    Config {
        detail: String,
        #[source]
        source: BoxError,
    },
    #[error("Conflict: {detail}")]
    Conflict {
        code: ErrorCode,
        detail: String,
        extensions: Option<serde_json::Value>,
    },
    #[error("Database unavailable: {reason}")]
    DbUnavailable {
        reason: String,
        #[source]
        source: BoxError,
        retry_after_secs: Option<u32>,
    },
    #[error("Timeout: {detail}")]
    Timeout {
        code: ErrorCode,
        detail: String,
        #[source]
        source: BoxError,
    },
}

impl AppError {
    /// Helper method to extract error code from any error variant
    pub fn code(&self) -> ErrorCode {
        match self {
            AppError::Validation { code, .. } => *code,
            AppError::Db { .. } => ErrorCode::DbError,
            AppError::NotFound { code, .. } => *code,
            AppError::Unauthorized => ErrorCode::Unauthorized,
            AppError::UnauthorizedMissingBearer => ErrorCode::UnauthorizedMissingBearer,
            AppError::UnauthorizedInvalidJwt => ErrorCode::UnauthorizedInvalidJwt,
            AppError::UnauthorizedExpiredJwt => ErrorCode::UnauthorizedExpiredJwt,
            AppError::Forbidden { code, .. } => *code,
            AppError::ForbiddenUserNotFound => ErrorCode::ForbiddenUserNotFound,
            // EmailNotAllowed is handled via Forbidden variant with EmailNotAllowed code
            AppError::BadRequest { code, .. } => *code,
            AppError::Internal { code, .. } => *code,
            AppError::Config { .. } => ErrorCode::ConfigError,
            AppError::Conflict { code, .. } => *code,
            AppError::DbUnavailable { .. } => ErrorCode::DbUnavailable,
            AppError::Timeout { code, .. } => *code,
        }
    }

    /// Helper method to extract error detail from any error variant
    fn detail(&self) -> String {
        match self {
            AppError::Validation { detail, .. } => detail.clone(),
            AppError::Db { detail, .. } => detail.clone(),
            AppError::NotFound { detail, .. } => detail.clone(),
            AppError::Unauthorized => "Authentication required".to_string(),
            AppError::UnauthorizedMissingBearer => "Missing or malformed Bearer token".to_string(),
            AppError::UnauthorizedInvalidJwt => "Invalid JWT".to_string(),
            AppError::UnauthorizedExpiredJwt => "Token expired".to_string(),
            AppError::Forbidden { detail, .. } => detail.clone(),
            AppError::ForbiddenUserNotFound => {
                // Avoid leaking whether a user record exists; present a generic message
                // to the client while preserving specifics in logs.
                "Access denied".to_string()
            }
            AppError::BadRequest { detail, .. } => detail.clone(),
            AppError::Internal { detail, .. } => detail.clone(),
            AppError::Config { detail, .. } => detail.clone(),
            AppError::Conflict { detail, .. } => detail.clone(),
            AppError::DbUnavailable { reason, .. } => reason.clone(),
            AppError::Timeout { detail, .. } => detail.clone(),
        }
    }

    /// Get the HTTP status code for this error
    pub fn status(&self) -> StatusCode {
        match self {
            AppError::Validation { status, .. } => *status,
            AppError::Db { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound { .. } => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::UnauthorizedMissingBearer => StatusCode::UNAUTHORIZED,
            AppError::UnauthorizedInvalidJwt => StatusCode::UNAUTHORIZED,
            AppError::UnauthorizedExpiredJwt => StatusCode::UNAUTHORIZED,
            AppError::Forbidden { .. } => StatusCode::FORBIDDEN,
            AppError::ForbiddenUserNotFound => StatusCode::UNAUTHORIZED,
            AppError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            AppError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Config { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Conflict { .. } => StatusCode::CONFLICT,
            AppError::DbUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Timeout { .. } => StatusCode::GATEWAY_TIMEOUT,
        }
    }

    pub fn invalid(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self::Validation {
            code,
            detail: detail.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn bad_request(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self::BadRequest {
            code,
            detail: detail.into(),
        }
    }

    pub fn precondition_required(detail: impl Into<String>) -> Self {
        Self::Validation {
            code: ErrorCode::PreconditionRequired,
            detail: detail.into(),
            status: StatusCode::PRECONDITION_REQUIRED,
        }
    }

    pub fn not_found(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self::NotFound {
            code,
            detail: detail.into(),
        }
    }

    /// Create a database error with source
    pub fn db(
        detail: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Db {
            detail: detail.into(),
            source: Box::new(source),
        }
    }

    /// Create an internal error with source
    pub fn internal(
        code: ErrorCode,
        detail: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Internal {
            code,
            detail: detail.into(),
            source: Box::new(source),
        }
    }

    pub fn unauthorized() -> Self {
        Self::Unauthorized
    }

    pub fn unauthorized_missing_bearer() -> Self {
        Self::UnauthorizedMissingBearer
    }

    pub fn unauthorized_invalid_jwt() -> Self {
        Self::UnauthorizedInvalidJwt
    }

    pub fn unauthorized_expired_jwt() -> Self {
        Self::UnauthorizedExpiredJwt
    }

    pub fn forbidden() -> Self {
        Self::Forbidden {
            code: ErrorCode::Forbidden,
            detail: "Access denied".to_string(),
        }
    }

    pub fn forbidden_with_code(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self::Forbidden {
            code,
            detail: detail.into(),
        }
    }

    pub fn forbidden_user_not_found() -> Self {
        Self::ForbiddenUserNotFound
    }

    pub fn email_not_allowed() -> Self {
        Self::Forbidden {
            code: ErrorCode::EmailNotAllowed,
            detail: "Access restricted. Please contact support if you believe this is an error."
                .to_string(),
        }
    }

    /// Create a configuration error with source
    pub fn config(
        detail: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Config {
            detail: detail.into(),
            source: Box::new(source),
        }
    }

    pub fn conflict(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self::Conflict {
            code,
            detail: detail.into(),
            extensions: None,
        }
    }

    pub fn conflict_with_extensions(
        code: ErrorCode,
        detail: impl Into<String>,
        extensions: serde_json::Value,
    ) -> Self {
        Self::Conflict {
            code,
            detail: detail.into(),
            extensions: Some(extensions),
        }
    }

    /// Create a db unavailable error (503) with optional retry_after_secs
    pub fn db_unavailable(
        reason: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
        retry_after_secs: Option<u32>,
    ) -> Self {
        Self::DbUnavailable {
            reason: reason.into(),
            source: Box::new(source),
            retry_after_secs,
        }
    }

    /// Create a timeout error with source (504)
    pub fn timeout(
        code: ErrorCode,
        detail: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Timeout {
            code,
            detail: detail.into(),
            source: Box::new(source),
        }
    }

    /// Create an internal error with a sentinel source (no real underlying error)
    pub fn internal_msg(code: ErrorCode, detail: impl Into<String>, msg: &'static str) -> Self {
        AppError::internal(code, detail, Sentinel(msg))
    }

    /// Create a config error with a sentinel source (no real underlying error)
    pub fn config_msg(detail: impl Into<String>, msg: &'static str) -> Self {
        AppError::config(detail, Sentinel(msg))
    }

    /// Extract extensions from the error variant if present
    fn extensions(&self) -> Option<serde_json::Value> {
        match self {
            AppError::Conflict { extensions, .. } => extensions.clone(),
            _ => None,
        }
    }

    /// Parse lock version numbers from an optimistic lock error detail string
    /// Expects format: "...expected version X, actual version Y..."
    fn parse_versions(detail: &str) -> Option<serde_json::Value> {
        let expected_prefix = "expected version ";
        let actual_prefix = ", actual version ";

        let expected_start = detail.find(expected_prefix)?;
        let expected_str_start = expected_start + expected_prefix.len();
        let actual_start = detail.find(actual_prefix)?;
        let expected_str = &detail[expected_str_start..actual_start];

        let actual_str_start = actual_start + actual_prefix.len();
        let actual_end = detail[actual_str_start..]
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(detail.len() - actual_str_start);
        let actual_str = &detail[actual_str_start..actual_str_start + actual_end];

        let expected: i32 = expected_str.parse().ok()?;
        let actual: i32 = actual_str.parse().ok()?;

        Some(serde_json::json!({ "expected": expected, "actual": actual }))
    }

    /// Private canonical method for building ProblemDetails
    /// This is the single source of truth for ProblemDetails construction
    fn to_problem_details_with_trace_id(&self, trace_id: String) -> ProblemDetails {
        let status = self.status();
        let code = self.code().as_str();
        let detail = self.detail();
        let extensions = self.extensions();

        ProblemDetails {
            type_: format!("https://nommie.app/errors/{}", code.to_uppercase()),
            title: Self::humanize_code(code),
            status: status.as_u16(),
            detail,
            code: code.to_string(),
            trace_id,
            extensions,
        }
    }

    fn humanize_code(code: &str) -> String {
        code.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl From<std::env::VarError> for AppError {
    fn from(e: std::env::VarError) -> Self {
        AppError::config("environment variable missing", e)
    }
}

impl From<db_infra::DbInfraError> for AppError {
    fn from(e: db_infra::DbInfraError) -> Self {
        AppError::config_msg(e.to_string(), "database configuration error")
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(e: sea_orm::DbErr) -> Self {
        // Delegate to infra adapter, then into AppError via DomainError mapping
        let de = crate::infra::db_errors::map_db_err(e);
        AppError::from(de)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        let message = e.to_string();
        let code_owned = if let sqlx::Error::Database(db) = &e {
            db.code().map(|c| c.to_string())
        } else {
            None
        };
        let code = code_owned.as_deref();

        if let Some(secs) = classify_transient(code, &message) {
            let reason = if message.contains("database is locked") {
                "database is locked".to_string()
            } else {
                format!("postgres transient error: {}", code.unwrap_or("unknown"))
            };
            return AppError::db_unavailable(reason, e, Some(secs));
        }

        // Fallback: non-transient or unknown error
        AppError::db("database operation failed", e)
    }
}

impl From<crate::errors::domain::DomainError> for AppError {
    fn from(err: crate::errors::domain::DomainError) -> Self {
        use crate::errors::domain::{ConflictKind, InfraErrorKind, NotFoundKind};
        use crate::errors::ErrorCode;

        // Error mapping rationale:
        // - Infra(DataCorruption) → DATA_CORRUPTION for data integrity issues
        // - Infra(Other) → INTERNAL_ERROR for generic infrastructure failures
        // - Both map to HTTP 500 but with distinct error codes for better debugging

        match err {
            crate::errors::domain::DomainError::Validation(kind, detail) => {
                match kind {
                    crate::errors::domain::ValidationKind::InvalidGameId => AppError::BadRequest {
                        code: ErrorCode::InvalidGameId,
                        detail,
                    },
                    _ => {
                        let error_code = match kind {
                            crate::errors::domain::ValidationKind::InvalidBid => {
                                ErrorCode::InvalidBid
                            }
                            crate::errors::domain::ValidationKind::MustFollowSuit => {
                                ErrorCode::MustFollowSuit
                            }
                            crate::errors::domain::ValidationKind::CardNotInHand => {
                                ErrorCode::CardNotInHand
                            }
                            crate::errors::domain::ValidationKind::OutOfTurn => {
                                ErrorCode::OutOfTurn
                            }
                            crate::errors::domain::ValidationKind::PhaseMismatch => {
                                ErrorCode::PhaseMismatch
                            }
                            crate::errors::domain::ValidationKind::ParseCard => {
                                ErrorCode::ParseCard
                            }
                            crate::errors::domain::ValidationKind::InvalidTrumpConversion => {
                                ErrorCode::InvalidTrumpConversion
                            }
                            crate::errors::domain::ValidationKind::InvalidSeat => {
                                ErrorCode::InvalidSeat
                            }
                            crate::errors::domain::ValidationKind::InvalidEmail => {
                                ErrorCode::InvalidEmail
                            }
                            crate::errors::domain::ValidationKind::Other(_) => {
                                ErrorCode::ValidationError
                            }
                            _ => ErrorCode::ValidationError, // catch-all for any new variants
                        };
                        AppError::Validation {
                            code: error_code,
                            detail,
                            status: StatusCode::UNPROCESSABLE_ENTITY,
                        }
                    }
                }
            }
            crate::errors::domain::DomainError::Conflict(kind, detail) => {
                let code = match kind {
                    ConflictKind::SeatTaken => ErrorCode::SeatTaken,
                    ConflictKind::UniqueEmail => ErrorCode::UniqueEmail,
                    ConflictKind::OptimisticLock => ErrorCode::OptimisticLock,
                    ConflictKind::GoogleSubMismatch => ErrorCode::GoogleSubMismatch,
                    ConflictKind::Other(_) => ErrorCode::Conflict, // generic conflict fallback
                };

                // For OptimisticLock, try to extract version info from detail string
                let extensions = if matches!(kind, ConflictKind::OptimisticLock) {
                    // Parse "expected version X, actual version Y" from detail
                    Self::parse_versions(&detail)
                } else {
                    None
                };

                // Preserve original detail from DomainError (already client-safe and specific)
                AppError::Conflict {
                    code,
                    detail,
                    extensions,
                }
            }
            crate::errors::domain::DomainError::NotFound(kind, detail) => {
                let code = match kind {
                    NotFoundKind::User => ErrorCode::UserNotFound,
                    NotFoundKind::Game => ErrorCode::GameNotFound,
                    NotFoundKind::Player => ErrorCode::PlayerNotFound,
                    NotFoundKind::Membership => ErrorCode::NotAMember,
                    NotFoundKind::Other(_) => ErrorCode::NotFound,
                };
                // Preserve original detail for NotFound (they're already client-safe)
                AppError::NotFound { code, detail }
            }
            crate::errors::domain::DomainError::Infra(kind, detail) => {
                match kind {
                    InfraErrorKind::Timeout => {
                        let detail_for_source = detail.clone();
                        AppError::timeout(
                            ErrorCode::DbTimeout,
                            "operation timed out".to_string(),
                            DomainErrorWrapper(detail_for_source),
                        )
                    }
                    InfraErrorKind::DbUnavailable => {
                        let detail_for_source = detail.clone();
                        AppError::db_unavailable(
                            detail,
                            DomainErrorWrapper(detail_for_source),
                            Some(1),
                        )
                    }
                    InfraErrorKind::DataCorruption => {
                        let detail_for_source = detail.clone();
                        AppError::internal(
                            ErrorCode::DataCorruption,
                            "data corruption detected".to_string(),
                            DomainErrorWrapper(detail_for_source),
                        )
                    }
                    InfraErrorKind::Other(_) => {
                        // Use a generic wrapper for source to avoid duplication
                        AppError::internal(
                            ErrorCode::InternalError,
                            detail,
                            DomainErrorWrapper("infrastructure failure".to_string()),
                        )
                    }
                }
            }
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        self.status()
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status();
        let trace_id = trace_ctx::trace_id();
        let problem_details = self.to_problem_details_with_trace_id(trace_id.clone());

        // Log with appropriate level: domain=warn, infra=error
        match self {
            // Domain-level conditions
            AppError::Validation { .. }
            | AppError::Conflict { .. }
            | AppError::NotFound { .. }
            | AppError::Forbidden { .. }
            | AppError::ForbiddenUserNotFound
            | AppError::BadRequest { .. }
            | AppError::Unauthorized
            | AppError::UnauthorizedMissingBearer
            | AppError::UnauthorizedInvalidJwt
            | AppError::UnauthorizedExpiredJwt => {
                warn!(
                    code = %self.code(),
                    status = %status.as_u16(),
                    detail = %problem_details.detail,
                    "domain error"
                );
            }
            // Infra/system-level
            AppError::Db { .. }
            | AppError::DbUnavailable { .. }
            | AppError::Timeout { .. }
            | AppError::Internal { .. }
            | AppError::Config { .. } => {
                error!(
                    code = %self.code(),
                    status = %status.as_u16(),
                    detail = %problem_details.detail,
                    "infrastructure error"
                );
            }
        }

        // Build response with appropriate headers based on status code
        // Header rules (enforced in production, validated in tests):
        // - 401 Unauthorized: MUST have WWW-Authenticate: Bearer, MUST NOT have Retry-After
        // - 503 Service Unavailable: MUST have Retry-After, MUST NOT have WWW-Authenticate
        // - Other errors (400, 404, 409, etc.): MUST NOT have either header
        let mut builder = HttpResponse::build(status);
        builder.insert_header((CONTENT_TYPE, "application/problem+json"));
        builder.insert_header(("x-trace-id", trace_id));

        // Apply status-specific headers according to RFC 7235 and RFC 7231
        match status {
            StatusCode::UNAUTHORIZED => {
                // RFC 7235: 401 responses must include WWW-Authenticate
                builder.insert_header((WWW_AUTHENTICATE, "Bearer"));
                // Note: Retry-After is explicitly NOT set for 401
            }
            StatusCode::SERVICE_UNAVAILABLE => {
                // RFC 7231: 503 responses should include Retry-After
                // Use retry_after_secs from DbUnavailable, otherwise default to 1
                let retry_secs: u32 = match self {
                    AppError::DbUnavailable {
                        retry_after_secs, ..
                    } => retry_after_secs.unwrap_or(1),
                    _ => 1,
                };
                builder.insert_header((RETRY_AFTER, retry_secs.to_string()));
                // Note: WWW-Authenticate is explicitly NOT set for 503
            }
            StatusCode::GATEWAY_TIMEOUT => {
                // 504 responses do NOT include Retry-After or WWW-Authenticate
            }
            _ => {
                // Other status codes (400, 404, 409, etc.) do not require
                // WWW-Authenticate or Retry-After headers
            }
        }

        builder.json(problem_details)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_transient_sqlite_locked() {
        assert_eq!(
            classify_transient(None, "xyz database is locked abc"),
            Some(1)
        );
    }

    #[test]
    fn test_classify_transient_pg_codes() {
        for code in ["55P03", "40001", "40P01", "53300", "08006"] {
            assert_eq!(
                classify_transient(Some(code), "irrelevant"),
                Some(1),
                "code {code}"
            );
        }
        assert_eq!(classify_transient(Some("99999"), "irrelevant"), None);
    }

    #[test]
    fn test_response_headers_db_unavailable_retry_after() {
        #[derive(Debug)]
        struct Dummy;
        impl std::fmt::Display for Dummy {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "dummy")
            }
        }
        impl std::error::Error for Dummy {}

        let err = AppError::DbUnavailable {
            reason: "down".to_string(),
            source: Box::new(Dummy),
            retry_after_secs: Some(5),
        };
        let resp = err.error_response();
        assert_eq!(resp.status().as_u16(), 503);
        let hdr = resp
            .headers()
            .get(actix_web::http::header::RETRY_AFTER)
            .unwrap();
        assert_eq!(hdr.to_str().unwrap(), "5");

        let err2 = AppError::DbUnavailable {
            reason: "down".to_string(),
            source: Box::new(Dummy),
            retry_after_secs: None,
        };
        let resp2 = err2.error_response();
        assert_eq!(resp2.status().as_u16(), 503);
        let hdr2 = resp2
            .headers()
            .get(actix_web::http::header::RETRY_AFTER)
            .unwrap();
        assert_eq!(hdr2.to_str().unwrap(), "1");
    }

    #[test]
    fn test_response_headers_timeout_no_retry_after() {
        #[derive(Debug)]
        struct Dummy;
        impl std::fmt::Display for Dummy {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "dummy")
            }
        }
        impl std::error::Error for Dummy {}

        let err = AppError::timeout(ErrorCode::DbTimeout, "deadline exceeded", Dummy);
        let resp = err.error_response();
        assert_eq!(resp.status().as_u16(), 504);
        assert!(resp
            .headers()
            .get(actix_web::http::header::RETRY_AFTER)
            .is_none());
    }

    #[test]
    fn test_response_headers_unauthorized_www_authenticate() {
        let err = AppError::unauthorized();
        let resp = err.error_response();
        assert_eq!(resp.status().as_u16(), 401);
        let hdr = resp
            .headers()
            .get(actix_web::http::header::WWW_AUTHENTICATE)
            .unwrap();
        assert_eq!(hdr.to_str().unwrap(), "Bearer");
        assert!(resp
            .headers()
            .get(actix_web::http::header::RETRY_AFTER)
            .is_none());
    }

    #[test]
    fn test_source_chain_and_display_not_duplicated() {
        #[derive(Debug)]
        struct Dummy;
        impl std::fmt::Display for Dummy {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "inner-dummy")
            }
        }
        impl std::error::Error for Dummy {}

        let err = AppError::db("sqlx error: X", Dummy);
        let outer = format!("{err}");
        let src = std::error::Error::source(&err).unwrap();
        let inner = format!("{src}");
        assert_ne!(outer, inner);
    }

    #[test]
    fn test_sentinel_source_chain() {
        use std::error::Error;
        let err = AppError::internal_msg(
            ErrorCode::InternalError,
            "operation failed",
            "no real source available",
        );
        assert!(err.source().is_some());
        let outer = format!("{}", err);
        let inner = format!("{}", err.source().unwrap());
        assert_ne!(outer, inner, "outer and inner messages should differ");
    }
}
