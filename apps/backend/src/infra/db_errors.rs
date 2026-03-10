//! SeaORM -> DomainError translation helpers.
//!
//! Adapters should convert `sea_orm::DbErr` into `crate::errors::domain::DomainError`
//! here, and higher layers can then map `DomainError` to `AppError` via `From`.

use tracing::{error, warn};

use crate::errors::domain::{ConflictKind, DomainError, InfraErrorKind, NotFoundKind};
use crate::logging::pii::Redacted;

fn mentions_sqlstate(msg: &str, code: &str) -> bool {
    msg.contains(code) || msg.contains(&format!("SQLSTATE({code})"))
}

/// Extract the constraint spec from SQLite "UNIQUE constraint failed: ..." error messages.
/// SQLite uses "table.col1, table.col2" for composite unique constraints.
fn extract_sqlite_constraint_spec(error_msg: &str) -> Option<&str> {
    const PREFIX: &str = "UNIQUE constraint failed: ";
    error_msg
        .find(PREFIX)
        .map(|i| &error_msg[i + PREFIX.len()..])
}

/// Map SQLite constraint spec to domain-specific conflict errors.
/// Handles both single-column ("table.column") and composite ("table.col1, table.col2") formats.
fn map_sqlite_constraint_to_conflict(
    constraint_spec: &str,
) -> Option<(ConflictKind, &'static str)> {
    // Composite constraints: "user_auth_identities.provider, user_auth_identities.email"
    // Check for the distinguishing column in each constraint.
    if constraint_spec.contains("user_auth_identities.email")
        || constraint_spec.contains("user_credentials.email")
        || constraint_spec.contains("users.email")
    {
        return Some((ConflictKind::UniqueEmail, "Email already registered"));
    }
    if constraint_spec.contains("user_auth_identities.provider_user_id")
        || constraint_spec.contains("user_credentials.google_sub")
    {
        return Some((
            ConflictKind::Other("UniqueGoogleSub".into()),
            "Google account already linked to another user",
        ));
    }
    None
}

/// Map PostgreSQL constraint names to domain-specific conflict errors.
fn map_postgres_constraint_to_conflict(error_msg: &str) -> Option<(ConflictKind, &'static str)> {
    if error_msg.contains("ux_user_auth_identities_provider_email")
        || error_msg.contains("user_auth_identities_provider_email")
        || error_msg.contains("user_credentials_email_key")
        || error_msg.contains("users_email_key")
    {
        return Some((ConflictKind::UniqueEmail, "Email already registered"));
    }
    if error_msg.contains("ux_user_auth_identities_provider_provider_user_id")
        || error_msg.contains("user_auth_identities_provider_provider_user_id_key")
        || error_msg.contains("user_credentials_google_sub_key")
    {
        return Some((
            ConflictKind::Other("UniqueGoogleSub".into()),
            "Google account already linked to another user",
        ));
    }
    None
}

/// Translate a `DbErr` into a `DomainError` with sanitized, PII-safe detail.
pub fn map_db_err(e: sea_orm::DbErr) -> DomainError {
    let error_msg = e.to_string();

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
                    warn!(game_id, "Game not found");
                    return DomainError::not_found(
                        NotFoundKind::Game,
                        format!("Game {game_id} not found"),
                    );
                }
            }
            // Fallback if parsing fails
            warn!(raw_error = %Redacted(msg), "Failed to parse GAME_NOT_FOUND error");
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
            warn!("Optimistic lock conflict detected (version info unavailable)");
            return DomainError::conflict(
                ConflictKind::OptimisticLock,
                "Resource was modified by another transaction; please retry",
            );
        }
        sea_orm::DbErr::ConnectionAcquire(_) | sea_orm::DbErr::Conn(_) => {
            // Connection-level failures when talking to the database. These are
            // surfaced as infrastructure "DB unavailable" errors at the domain
            // layer and ultimately map to AppError::DbUnavailable/DB_UNAVAILABLE.
            // We log explicitly here so we can correlate request spans that hit
            // connection pool timeouts with the higher-level AppError handling.
            warn!(
                raw_error = %Redacted(&error_msg),
                "db_errors: mapping connection error to Infra(DbUnavailable)"
            );
            return DomainError::infra(InfraErrorKind::DbUnavailable, "Database unavailable");
        }
        _ => {}
    }

    if mentions_sqlstate(&error_msg, "23505")
        || error_msg.contains("duplicate key value violates unique constraint")
        || error_msg.contains("UNIQUE constraint failed")
    {
        warn!(raw_error = %Redacted(&error_msg), "Unique constraint violation");

        // Try to extract constraint spec from SQLite format errors first
        if let Some(constraint_spec) = extract_sqlite_constraint_spec(&error_msg) {
            if let Some((kind, detail)) = map_sqlite_constraint_to_conflict(constraint_spec) {
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
        warn!(raw_error = %Redacted(&error_msg), "Foreign key constraint violation");
        return DomainError::validation_other("Foreign key constraint violation");
    }

    if mentions_sqlstate(&error_msg, "23514") {
        warn!(raw_error = %Redacted(&error_msg), "Check constraint violation");
        return DomainError::validation_other("Check constraint violation");
    }

    if error_msg.contains("timeout")
        || error_msg.contains("pool")
        || error_msg.contains("unavailable")
    {
        warn!(raw_error = %Redacted(&error_msg), "Database timeout or pool issue");
        return DomainError::infra(InfraErrorKind::Timeout, "Database timeout");
    }

    error!(raw_error = %Redacted(&error_msg), "Unhandled database error");
    DomainError::infra(
        InfraErrorKind::Other("DbErr".into()),
        "Database operation failed",
    )
}
