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
use crate::web::trace_ctx;

// (legacy helper removed; DB error mapping lives in infra::db_errors)

#[derive(Serialize)]
pub struct ProblemDetails {
    #[serde(rename = "type")]
    pub type_: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    pub code: String,
    pub trace_id: String,
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
    Db { detail: String },
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
    Internal { code: ErrorCode, detail: String },
    #[error("Configuration error: {detail}")]
    Config { detail: String },
    #[error("Conflict: {detail}")]
    Conflict { code: ErrorCode, detail: String },
    #[error("Database unavailable")]
    DbUnavailable,
    #[error("Timeout: {detail}")]
    Timeout { code: ErrorCode, detail: String },
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
            AppError::BadRequest { code, .. } => *code,
            AppError::Internal { code, .. } => *code,
            AppError::Config { .. } => ErrorCode::ConfigError,
            AppError::Conflict { code, .. } => *code,
            AppError::DbUnavailable => ErrorCode::DbUnavailable,
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
            AppError::ForbiddenUserNotFound => "User not found in database".to_string(),
            AppError::BadRequest { detail, .. } => detail.clone(),
            AppError::Internal { detail, .. } => detail.clone(),
            AppError::Config { detail, .. } => detail.clone(),
            AppError::Conflict { detail, .. } => detail.clone(),
            AppError::DbUnavailable => "Database unavailable".to_string(),
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
            AppError::ForbiddenUserNotFound => StatusCode::FORBIDDEN,
            AppError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            AppError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Config { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Conflict { .. } => StatusCode::CONFLICT,
            AppError::DbUnavailable => StatusCode::SERVICE_UNAVAILABLE,
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

    pub fn internal(detail: impl Into<String>) -> Self {
        Self::Internal {
            code: ErrorCode::InternalError,
            detail: detail.into(),
        }
    }

    pub fn bad_request(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self::BadRequest {
            code,
            detail: detail.into(),
        }
    }

    pub fn not_found(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self::NotFound {
            code,
            detail: detail.into(),
        }
    }

    pub fn db(detail: impl Into<String>) -> Self {
        Self::Db {
            detail: detail.into(),
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

    pub fn config(detail: impl Into<String>) -> Self {
        Self::Config {
            detail: detail.into(),
        }
    }

    pub fn conflict(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self::Conflict {
            code,
            detail: detail.into(),
        }
    }

    pub fn db_unavailable() -> Self {
        Self::DbUnavailable
    }

    pub fn timeout(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self::Timeout {
            code,
            detail: detail.into(),
        }
    }

    /// Private canonical method for building ProblemDetails
    /// This is the single source of truth for ProblemDetails construction
    fn to_problem_details_with_trace_id(&self, trace_id: String) -> ProblemDetails {
        let status = self.status();
        let code = self.code().as_str();
        let detail = self.detail();

        ProblemDetails {
            type_: format!("https://nommie.app/errors/{}", code.to_uppercase()),
            title: Self::humanize_code(code),
            status: status.as_u16(),
            detail,
            code: code.to_string(),
            trace_id,
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
        AppError::Config {
            detail: format!("env var error: {e}"),
        }
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(e: sea_orm::DbErr) -> Self {
        // Delegate to infra adapter, then into AppError via DomainError mapping
        let de = crate::infra::db_errors::map_db_err(e);
        AppError::from(de)
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
            crate::errors::domain::DomainError::Validation(detail) => AppError::Validation {
                code: ErrorCode::ValidationError,
                detail,
                status: StatusCode::UNPROCESSABLE_ENTITY,
            },
            crate::errors::domain::DomainError::Conflict(kind, detail) => {
                let code = match kind {
                    ConflictKind::SeatTaken => ErrorCode::SeatTaken,
                    ConflictKind::UniqueEmail => ErrorCode::UniqueEmail,
                    ConflictKind::OptimisticLock => ErrorCode::OptimisticLock,
                    ConflictKind::JoinCodeConflict => ErrorCode::JoinCodeConflict,
                    ConflictKind::Other(_) => ErrorCode::Conflict, // generic conflict fallback
                };
                AppError::Conflict { code, detail }
            }
            crate::errors::domain::DomainError::NotFound(kind, detail) => {
                let code = match kind {
                    NotFoundKind::User => ErrorCode::UserNotFound,
                    NotFoundKind::Game => ErrorCode::GameNotFound,
                    NotFoundKind::Other(ref s) if s == "Player" => ErrorCode::PlayerNotFound,
                    NotFoundKind::Other(_) => ErrorCode::NotFound,
                };
                AppError::NotFound { code, detail }
            }
            crate::errors::domain::DomainError::Infra(kind, detail) => match kind {
                InfraErrorKind::Timeout => AppError::timeout(ErrorCode::DbTimeout, detail),
                InfraErrorKind::DbUnavailable => AppError::DbUnavailable,
                InfraErrorKind::DataCorruption => AppError::Internal {
                    code: ErrorCode::DataCorruption,
                    detail,
                },
                InfraErrorKind::Other(_) => AppError::Internal {
                    code: ErrorCode::InternalError,
                    detail,
                },
            },
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
                    trace_id = %trace_id,
                    code = %self.code(),
                    status = %status.as_u16(),
                    detail = %problem_details.detail,
                    "domain error"
                );
            }
            // Infra/system-level
            AppError::Db { .. }
            | AppError::DbUnavailable
            | AppError::Timeout { .. }
            | AppError::Internal { .. }
            | AppError::Config { .. } => {
                error!(
                    trace_id = %trace_id,
                    code = %self.code(),
                    status = %status.as_u16(),
                    detail = %problem_details.detail,
                    "infrastructure error"
                );
            }
        }

        let is_unauthorized = matches!(
            self,
            AppError::Unauthorized
                | AppError::UnauthorizedMissingBearer
                | AppError::UnauthorizedInvalidJwt
                | AppError::UnauthorizedExpiredJwt
        );

        let is_service_unavailable = status == StatusCode::SERVICE_UNAVAILABLE;

        // Build step-by-step to avoid borrowing a temporary
        let mut builder = HttpResponse::build(status);
        builder.insert_header((CONTENT_TYPE, "application/problem+json"));
        builder.insert_header(("x-trace-id", trace_id)); // keep custom
        if is_unauthorized {
            builder.insert_header((WWW_AUTHENTICATE, "Bearer"));
        }
        if is_service_unavailable {
            builder.insert_header((RETRY_AFTER, "1"));
        }

        builder.json(problem_details)
    }
}
