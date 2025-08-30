use actix_web::{error::ResponseError, http::StatusCode, HttpMessage, HttpRequest, HttpResponse};
use serde::Serialize;
use thiserror::Error;

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
        code: &'static str,
        detail: String,
        status: StatusCode,
        trace_id: Option<String>,
    },
    #[error("Database error: {detail}")]
    Db {
        detail: String,
        trace_id: Option<String>,
    },
    #[error("Not found: {detail}")]
    NotFound {
        code: &'static str,
        detail: String,
        trace_id: Option<String>,
    },
    #[error("Unauthorized")]
    Unauthorized { trace_id: Option<String> },
    #[error("Forbidden")]
    Forbidden { trace_id: Option<String> },
    #[error("Bad request: {detail}")]
    BadRequest {
        code: &'static str,
        detail: String,
        trace_id: Option<String>,
    },
    #[error("Internal error: {detail}")]
    Internal {
        detail: String,
        trace_id: Option<String>,
    },
}

impl AppError {
    /// Ensures that every error response has a trace_id.
    /// If no trace_id is provided, sets it to "unknown".
    pub fn ensure_trace_id(err: AppError) -> AppError {
        let trace_id = err.trace_id().unwrap_or_else(|| "unknown".to_string());
        err.with_trace_id(Some(trace_id))
    }

    /// Helper method to extract trace_id from any error variant
    fn trace_id(&self) -> Option<String> {
        match self {
            AppError::Validation { trace_id, .. } => trace_id.clone(),
            AppError::Db { trace_id, .. } => trace_id.clone(),
            AppError::NotFound { trace_id, .. } => trace_id.clone(),
            AppError::Unauthorized { trace_id } => trace_id.clone(),
            AppError::Forbidden { trace_id } => trace_id.clone(),
            AppError::BadRequest { trace_id, .. } => trace_id.clone(),
            AppError::Internal { trace_id, .. } => trace_id.clone(),
        }
    }

    /// Helper method to extract error code from any error variant
    fn code(&self) -> String {
        match self {
            AppError::Validation { code, .. } => code.to_string(),
            AppError::Db { .. } => "DB_ERROR".to_string(),
            AppError::NotFound { code, .. } => code.to_string(),
            AppError::Unauthorized { .. } => "UNAUTHORIZED".to_string(),
            AppError::Forbidden { .. } => "FORBIDDEN".to_string(),
            AppError::BadRequest { code, .. } => code.to_string(),
            AppError::Internal { .. } => "INTERNAL".to_string(),
        }
    }

    /// Helper method to extract error detail from any error variant
    fn detail(&self) -> String {
        match self {
            AppError::Validation { detail, .. } => detail.clone(),
            AppError::Db { detail, .. } => detail.clone(),
            AppError::NotFound { detail, .. } => detail.clone(),
            AppError::Unauthorized { .. } => "Authentication required".to_string(),
            AppError::Forbidden { .. } => "Access denied".to_string(),
            AppError::BadRequest { detail, .. } => detail.clone(),
            AppError::Internal { detail, .. } => detail.clone(),
        }
    }

    /// Get the HTTP status code for this error
    pub fn status(&self) -> StatusCode {
        match self {
            AppError::Validation { status, .. } => *status,
            AppError::Db { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound { .. } => StatusCode::NOT_FOUND,
            AppError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            AppError::Forbidden { .. } => StatusCode::FORBIDDEN,
            AppError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            AppError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn invalid(code: &'static str, detail: String) -> Self {
        Self::Validation {
            code,
            detail,
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    pub fn internal(detail: String) -> Self {
        Self::Internal {
            detail,
            trace_id: None,
        }
    }

    pub fn bad_request(code: &'static str, detail: String) -> Self {
        Self::BadRequest {
            code,
            detail,
            trace_id: None,
        }
    }

    pub fn not_found(code: &'static str, detail: String) -> Self {
        Self::NotFound {
            code,
            detail,
            trace_id: None,
        }
    }

    pub fn db(detail: String) -> Self {
        Self::Db {
            detail,
            trace_id: None,
        }
    }

    pub fn unauthorized() -> Self {
        Self::Unauthorized { trace_id: None }
    }

    pub fn forbidden() -> Self {
        Self::Forbidden { trace_id: None }
    }

    pub fn with_trace_id(self, trace_id: Option<String>) -> Self {
        match self {
            AppError::Validation {
                code,
                detail,
                status,
                ..
            } => AppError::Validation {
                code,
                detail,
                status,
                trace_id,
            },
            AppError::Db { detail, .. } => AppError::Db { detail, trace_id },
            AppError::NotFound { code, detail, .. } => AppError::NotFound {
                code,
                detail,
                trace_id,
            },
            AppError::Unauthorized { .. } => AppError::Unauthorized { trace_id },
            AppError::Forbidden { .. } => AppError::Forbidden { trace_id },
            AppError::BadRequest { code, detail, .. } => AppError::BadRequest {
                code,
                detail,
                trace_id,
            },
            AppError::Internal { detail, .. } => AppError::Internal { detail, trace_id },
        }
    }

    pub fn from_req(req: &HttpRequest, err: AppError) -> AppError {
        let trace_id = req.extensions().get::<String>().cloned();
        err.with_trace_id(trace_id)
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

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        self.status()
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status();
        let code = self.code();
        let detail = self.detail();
        let trace_id = self.trace_id().unwrap_or_else(|| "unknown".to_string());

        let problem_details = ProblemDetails {
            type_: format!("https://nommie.app/errors/{}", code.to_uppercase()),
            title: Self::humanize_code(&code),
            status: status.as_u16(),
            detail,
            code,
            trace_id,
        };

        HttpResponse::build(status)
            .content_type("application/problem+json")
            .json(problem_details)
    }
}
