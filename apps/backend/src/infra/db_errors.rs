//! SeaORM -> DomainError translation helpers.
//!
//! Adapters should convert `sea_orm::DbErr` into `crate::errors::domain::DomainError`
//! here, and higher layers can then map `DomainError` to `AppError` via `From`.

use tracing::{error, warn};

use crate::errors::domain::{ConflictKind, DomainError, InfraErrorKind, NotFoundKind};
use crate::logging::pii::Redacted;
use crate::web::trace_ctx;

fn mentions_sqlstate(msg: &str, code: &str) -> bool {
    msg.contains(code) || msg.contains(&format!("SQLSTATE({code})"))
}

/// Translate a `DbErr` into a `DomainError` with sanitized, PII-safe detail.
pub fn map_db_err(e: sea_orm::DbErr) -> DomainError {
    let error_msg = e.to_string();
    let trace_id = trace_ctx::trace_id();

    match &e {
        sea_orm::DbErr::RecordNotFound(_) => {
            return DomainError::not_found(
                NotFoundKind::Other("Record".into()),
                "Record not found",
            );
        }
        sea_orm::DbErr::Custom(msg) if msg.starts_with("OPTIMISTIC_LOCK:") => {
            warn!(trace_id = %trace_id, "Optimistic lock conflict detected");
            return DomainError::conflict(
                ConflictKind::OptimisticLock,
                "Resource was modified by another transaction; please retry",
            );
        }
        sea_orm::DbErr::ConnectionAcquire(_) | sea_orm::DbErr::Conn(_) => {
            warn!(trace_id = %trace_id, raw_error = %Redacted(&error_msg), "Database unavailable");
            return DomainError::infra(InfraErrorKind::DbUnavailable, "Database unavailable");
        }
        _ => {}
    }

    if mentions_sqlstate(&error_msg, "23505")
        || error_msg.contains("duplicate key value violates unique constraint")
    {
        warn!(trace_id = %trace_id, raw_error = %Redacted(&error_msg), "Unique constraint violation");
        // Specific mapping for user_credentials.email
        if error_msg.contains("user_credentials_email_key") || error_msg.contains("users_email_key")
        {
            return DomainError::conflict(ConflictKind::UniqueEmail, "Email already registered");
        }
        // Specific mapping for user_credentials.google_sub
        if error_msg.contains("user_credentials_google_sub_key") {
            return DomainError::conflict(
                ConflictKind::Other("UniqueGoogleSub".into()),
                "Google account already linked to another user",
            );
        }
        // Specific mapping for games.join_code
        if error_msg.contains("games_join_code_key") {
            return DomainError::conflict(
                ConflictKind::JoinCodeConflict,
                "Join code already exists",
            );
        }
        return DomainError::conflict(
            ConflictKind::Other("Unique".into()),
            "Unique constraint violation",
        );
    }

    if mentions_sqlstate(&error_msg, "23503") {
        warn!(trace_id = %trace_id, raw_error = %Redacted(&error_msg), "Foreign key constraint violation");
        return DomainError::validation_other("Foreign key constraint violation");
    }

    if mentions_sqlstate(&error_msg, "23514") {
        warn!(trace_id = %trace_id, raw_error = %Redacted(&error_msg), "Check constraint violation");
        return DomainError::validation_other("Check constraint violation");
    }

    if error_msg.contains("timeout")
        || error_msg.contains("pool")
        || error_msg.contains("unavailable")
    {
        warn!(trace_id = %trace_id, raw_error = %Redacted(&error_msg), "Database timeout or pool issue");
        return DomainError::infra(InfraErrorKind::Timeout, "Database timeout");
    }

    error!(trace_id = %trace_id, raw_error = %Redacted(&error_msg), "Unhandled database error");
    DomainError::infra(
        InfraErrorKind::Other("DbErr".into()),
        "Database operation failed",
    )
}
