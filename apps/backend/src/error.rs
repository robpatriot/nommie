use actix_web::error::ResponseError;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::errors::ErrorCode;
use crate::web::trace_ctx;

#[derive(Serialize, Deserialize)]
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
    fn code(&self) -> String {
        match self {
            AppError::Validation { code, .. } => code.as_str().to_string(),
            AppError::Db { .. } => ErrorCode::DbError.as_str().to_string(),
            AppError::NotFound { code, .. } => code.as_str().to_string(),
            AppError::Unauthorized => ErrorCode::Unauthorized.as_str().to_string(),
            AppError::UnauthorizedMissingBearer => {
                ErrorCode::UnauthorizedMissingBearer.as_str().to_string()
            }
            AppError::UnauthorizedInvalidJwt => {
                ErrorCode::UnauthorizedInvalidJwt.as_str().to_string()
            }
            AppError::UnauthorizedExpiredJwt => {
                ErrorCode::UnauthorizedExpiredJwt.as_str().to_string()
            }
            AppError::Forbidden => ErrorCode::Forbidden.as_str().to_string(),
            AppError::ForbiddenUserNotFound => {
                ErrorCode::ForbiddenUserNotFound.as_str().to_string()
            }
            AppError::BadRequest { code, .. } => code.as_str().to_string(),
            AppError::Internal { .. } => ErrorCode::Internal.as_str().to_string(),
            AppError::Config { .. } => ErrorCode::ConfigError.as_str().to_string(),
            AppError::Conflict { code, .. } => code.as_str().to_string(),
            AppError::DbUnavailable => ErrorCode::DbUnavailable.as_str().to_string(),
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
            AppError::DbUnavailable => StatusCode::INTERNAL_SERVER_ERROR,
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
        let code = self.code();
        let detail = self.detail();

        ProblemDetails {
            type_: format!("https://nommie.app/errors/{}", code.to_uppercase()),
            title: Self::humanize_code(&code),
            status: status.as_u16(),
            detail,
            code,
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
        AppError::internal(format!("env var error: {e}"))
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(e: sea_orm::DbErr) -> Self {
        AppError::internal(format!("db error: {e}"))
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

        HttpResponse::build(status)
            .content_type("application/problem+json")
            .insert_header(("x-trace-id", trace_id))
            .json(problem_details)
    }
}
