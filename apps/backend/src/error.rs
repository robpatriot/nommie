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
use tracing::warn;

use crate::errors::ErrorCode;
use crate::web::trace_ctx;

/// Helper function to detect SQLSTATE codes in error messages
fn mentions_sqlstate(msg: &str, code: &str) -> bool {
    msg.contains(code) || msg.contains(&format!("SQLSTATE({code})"))
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
    #[error("Forbidden")]
    Forbidden,
    #[error("Forbidden: User not found")]
    ForbiddenUserNotFound,
    #[error("Bad request: {detail}")]
    BadRequest { code: ErrorCode, detail: String },
    #[error("Internal error: {detail}")]
    Internal { detail: String },
    #[error("Configuration error: {detail}")]
    Config { detail: String },
    #[error("Conflict: {detail}")]
    Conflict { code: ErrorCode, detail: String },
    #[error("Database unavailable")]
    DbUnavailable,
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
            AppError::Forbidden => ErrorCode::Forbidden,
            AppError::ForbiddenUserNotFound => ErrorCode::ForbiddenUserNotFound,
            AppError::BadRequest { code, .. } => *code,
            AppError::Internal { .. } => ErrorCode::Internal,
            AppError::Config { .. } => ErrorCode::ConfigError,
            AppError::Conflict { code, .. } => *code,
            AppError::DbUnavailable => ErrorCode::DbUnavailable,
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
            AppError::Forbidden => "Access denied".to_string(),
            AppError::ForbiddenUserNotFound => "User not found in database".to_string(),
            AppError::BadRequest { detail, .. } => detail.clone(),
            AppError::Internal { detail, .. } => detail.clone(),
            AppError::Config { detail, .. } => detail.clone(),
            AppError::Conflict { detail, .. } => detail.clone(),
            AppError::DbUnavailable => "Database unavailable".to_string(),
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
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::ForbiddenUserNotFound => StatusCode::FORBIDDEN,
            AppError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            AppError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Config { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Conflict { .. } => StatusCode::CONFLICT,
            AppError::DbUnavailable => StatusCode::SERVICE_UNAVAILABLE,
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
        Self::Forbidden
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
        let error_msg = e.to_string();
        let trace_id = trace_ctx::trace_id();

        // Handle structured SeaORM error types first
        match &e {
            sea_orm::DbErr::RecordNotFound(_) => {
                return AppError::NotFound {
                    code: ErrorCode::RecordNotFound,
                    detail: error_msg,
                };
            }
            sea_orm::DbErr::ConnectionAcquire(_) => {
                warn!(
                    trace_id = %trace_id,
                    raw_error = %error_msg,
                    "Database connection acquire failed"
                );
                return AppError::DbUnavailable;
            }
            sea_orm::DbErr::Conn(_) => {
                warn!(
                    trace_id = %trace_id,
                    raw_error = %error_msg,
                    "Database connection failed"
                );
                return AppError::DbUnavailable;
            }
            _ => {}
        }

        // Check for SQLSTATE codes using helper function
        if mentions_sqlstate(&error_msg, "23505")
            || error_msg.contains("duplicate key value violates unique constraint")
        {
            warn!(
                trace_id = %trace_id,
                raw_error = %error_msg,
                "Unique constraint violation detected"
            );
            return AppError::Conflict {
                code: ErrorCode::UniqueViolation,
                detail: "Unique constraint violation".to_string(),
            };
        }

        if mentions_sqlstate(&error_msg, "23503") {
            warn!(
                trace_id = %trace_id,
                raw_error = %error_msg,
                "Foreign key constraint violation detected"
            );
            return AppError::Conflict {
                code: ErrorCode::FkViolation,
                detail: "Foreign key constraint violation".to_string(),
            };
        }

        if mentions_sqlstate(&error_msg, "23514") {
            warn!(
                trace_id = %trace_id,
                raw_error = %error_msg,
                "Check constraint violation detected"
            );
            return AppError::BadRequest {
                code: ErrorCode::CheckViolation,
                detail: "Check constraint violation".to_string(),
            };
        }

        // Check for connection/pool issues via string matching as fallback
        if error_msg.contains("connection")
            || error_msg.contains("timeout")
            || error_msg.contains("pool")
            || error_msg.contains("unavailable")
        {
            warn!(
                trace_id = %trace_id,
                raw_error = %error_msg,
                "Database connection issue detected"
            );
            return AppError::DbUnavailable;
        }

        // Fallback: generic database error
        warn!(
            trace_id = %trace_id,
            raw_error = %error_msg,
            "Unhandled database error"
        );
        AppError::Db {
            detail: "Database operation failed".to_string(),
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
