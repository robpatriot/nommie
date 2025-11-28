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

/// Extract table.column from SQLite "UNIQUE constraint failed: table.column" error messages.
fn extract_sqlite_table_column(error_msg: &str) -> Option<&str> {
    // SQLite format: "UNIQUE constraint failed: table.column"
    if let Some(prefix) = error_msg.find("UNIQUE constraint failed: ") {
        let rest = &error_msg[prefix + "UNIQUE constraint failed: ".len()..];
        // Take up to the end or first space/newline/quote
        let table_column = rest
            .split_whitespace()
            .next()
            .or_else(|| rest.split('\n').next())
            .or_else(|| rest.split('"').next());
        return table_column;
    }
    None
}

/// Map SQLite table.column format to domain-specific conflict errors.
fn map_sqlite_table_column_to_conflict(table_column: &str) -> Option<(ConflictKind, &'static str)> {
    match table_column {
        "user_credentials.email" | "users.email" => {
            Some((ConflictKind::UniqueEmail, "Email already registered"))
        }
        "user_credentials.google_sub" => Some((
            ConflictKind::Other("UniqueGoogleSub".into()),
            "Google account already linked to another user",
        )),
        "games.join_code" => Some((ConflictKind::JoinCodeConflict, "Join code already exists")),
        _ => None,
    }
}

/// Map PostgreSQL constraint names to domain-specific conflict errors.
fn map_postgres_constraint_to_conflict(error_msg: &str) -> Option<(ConflictKind, &'static str)> {
    if error_msg.contains("user_credentials_email_key") || error_msg.contains("users_email_key") {
        return Some((ConflictKind::UniqueEmail, "Email already registered"));
    }
    if error_msg.contains("user_credentials_google_sub_key") {
        return Some((
            ConflictKind::Other("UniqueGoogleSub".into()),
            "Google account already linked to another user",
        ));
    }
    if error_msg.contains("games_join_code_key") {
        return Some((ConflictKind::JoinCodeConflict, "Join code already exists"));
    }
    None
}

/// Translate a `DbErr` into a `DomainError` with sanitized, PII-safe detail.
pub fn map_db_err(e: sea_orm::DbErr) -> DomainError {
    let error_msg = e.to_string();
    let trace_id = trace_ctx::trace_id();

    match &e {
        sea_orm::DbErr::RecordNotFound(_) => {
            // Generic record not found
            return DomainError::not_found(
                NotFoundKind::Other("Record".into()),
                "Record not found",
            );
        }
        sea_orm::DbErr::Custom(msg) if msg.starts_with("GAME_NOT_FOUND:") => {
            // Structured game not found error from adapter layer
            if let Some(game_id_str) = msg.strip_prefix("GAME_NOT_FOUND:") {
                if let Ok(game_id) = game_id_str.parse::<i64>() {
                    warn!(trace_id = %trace_id, game_id, "Game not found");
                    return DomainError::not_found(
                        NotFoundKind::Game,
                        format!("Game {game_id} not found"),
                    );
                }
            }
            // Fallback if parsing fails
            warn!(trace_id = %trace_id, raw_error = %Redacted(msg), "Failed to parse GAME_NOT_FOUND error");
            return DomainError::not_found(NotFoundKind::Game, "Game not found");
        }
        sea_orm::DbErr::Custom(msg) if msg.starts_with("OPTIMISTIC_LOCK:") => {
            // Try to parse structured version info
            if let Some(json_str) = msg.strip_prefix("OPTIMISTIC_LOCK:") {
                #[derive(serde::Deserialize)]
                struct LockInfo {
                    expected: i32,
                    actual: i32,
                }

                if let Ok(info) = serde_json::from_str::<LockInfo>(json_str) {
                    // Log with version details for observability
                    warn!(
                        trace_id = %trace_id,
                        expected = info.expected,
                        actual = info.actual,
                        "Optimistic lock conflict detected"
                    );

                    return DomainError::conflict(
                        ConflictKind::OptimisticLock,
                        format!(
                            "Resource was modified concurrently (expected version {}, actual version {}). Please refresh and retry.",
                            info.expected, info.actual
                        ),
                    );
                }
            }

            // Fallback for back-compat or parsing failures
            warn!(trace_id = %trace_id, "Optimistic lock conflict detected (version info unavailable)");
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
        || error_msg.contains("UNIQUE constraint failed")
    {
        warn!(trace_id = %trace_id, raw_error = %Redacted(&error_msg), "Unique constraint violation");

        // Try to extract table.column from SQLite format errors first
        if let Some(table_column) = extract_sqlite_table_column(&error_msg) {
            if let Some((kind, detail)) = map_sqlite_table_column_to_conflict(table_column) {
                return DomainError::conflict(kind, detail);
            }
        }

        // Check for PostgreSQL constraint name patterns
        if let Some((kind, detail)) = map_postgres_constraint_to_conflict(&error_msg) {
            return DomainError::conflict(kind, detail);
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
