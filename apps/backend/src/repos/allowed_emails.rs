//! Admission table repository for first-time login.

use sea_orm::{ConnectionTrait, DatabaseTransaction};
use unicode_normalization::UnicodeNormalization;

use crate::adapters::allowed_emails_sea;
use crate::errors::domain::DomainError;
use crate::state::admission_mode::AdmissionMode;

/// Normalize an email/pattern for consistent storage and lookup.
/// Canonical path used by ALLOWED_EMAILS, ADMIN_EMAILS, allow matching, exact admin matching.
pub fn normalize(value: &str) -> String {
    value.trim().nfkc().collect::<String>().to_lowercase()
}

/// Check if an email matches a pattern (supports `*` wildcards).
fn matches_pattern(email: &str, pattern: &str) -> bool {
    if !pattern.contains('*') {
        return email == pattern;
    }

    let parts: Vec<&str> = pattern.split('*').collect();

    if pattern.starts_with('*') {
        if parts.len() == 2 && parts[1].is_empty() {
            return true;
        }
        if parts.len() == 2 {
            return email.ends_with(parts[1]);
        }
    }

    if pattern.ends_with('*') {
        if parts.len() == 2 && parts[0].is_empty() {
            return true;
        }
        if parts.len() == 2 {
            return email.starts_with(parts[0]);
        }
    }

    let mut search_start = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }

        if i == 0 && !pattern.starts_with('*') {
            if !email.starts_with(part) {
                return false;
            }
            search_start = part.len();
        } else if i == parts.len() - 1 && !pattern.ends_with('*') {
            if !email.ends_with(part) {
                return false;
            }
        } else if let Some(pos) = email[search_start..].find(part) {
            search_start += pos + part.len();
        } else {
            return false;
        }
    }

    true
}

/// Check admission and admin status for an email.
///
/// Returns `(admitted, is_admin)`. Admission mode is deployment config (from ALLOWED_EMAILS at
/// startup), not inferred from DB.
///
/// - **Open**: Always admits; admin from exact row if present.
/// - **Restricted**: Admitted if exact match or wildcard match; admin only from exact row.
///
/// Uses indexed exact lookup first; loads wildcard rows only when exact match fails.
pub async fn check_admission_and_admin<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    email: &str,
    mode: AdmissionMode,
) -> Result<(bool, bool), DomainError> {
    let normalized = normalize(email);

    if mode == AdmissionMode::Open {
        let is_admin = allowed_emails_sea::find_by_email(conn, &normalized)
            .await
            .map_err(crate::infra::db_errors::map_db_err)?
            .is_some_and(|r| r.is_admin);
        return Ok((true, is_admin));
    }

    let exact = allowed_emails_sea::find_by_email(conn, &normalized)
        .await
        .map_err(crate::infra::db_errors::map_db_err)?;

    if let Some(rule) = exact {
        return Ok((true, rule.is_admin));
    }

    let wildcards = allowed_emails_sea::list_wildcard_rules(conn)
        .await
        .map_err(crate::infra::db_errors::map_db_err)?;

    let admitted = wildcards
        .iter()
        .any(|rule| matches_pattern(&normalized, &rule.pattern));

    Ok((admitted, false))
}

/// Parse ALLOWED_EMAILS from environment. Returns normalized patterns.
pub fn parse_allowed_emails_from_env() -> Vec<String> {
    let env_value = match std::env::var("ALLOWED_EMAILS") {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let trimmed = env_value.trim();
    if trimmed.is_empty() {
        return vec![];
    }

    trimmed
        .split(',')
        .map(|s| normalize(s.trim()))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse ADMIN_EMAILS from environment. Returns normalized exact emails only. Ignores wildcards.
pub fn parse_admin_emails_from_env() -> Vec<String> {
    let env_value = match std::env::var("ADMIN_EMAILS") {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let trimmed = env_value.trim();
    if trimmed.is_empty() {
        return vec![];
    }

    trimmed
        .split(',')
        .map(|s| normalize(s.trim()))
        .filter(|s| !s.is_empty())
        .filter(|s| !s.contains('*'))
        .collect()
}

/// Seed missing entries from ALLOWED_EMAILS into the admission table.
/// Additive and idempotent. New rows have is_admin=false.
pub async fn seed_from_env(txn: &DatabaseTransaction) -> Result<usize, DomainError> {
    let patterns = parse_allowed_emails_from_env();
    let mut inserted = 0;
    for pattern in patterns {
        if allowed_emails_sea::insert_if_not_exists(txn, &pattern, false)
            .await
            .map_err(crate::infra::db_errors::map_db_err)?
        {
            inserted += 1;
        }
    }
    Ok(inserted)
}

/// Augment from ADMIN_EMAILS: for each exact normalized email, upsert with is_admin=true.
/// Ignores wildcards. Idempotent.
pub async fn seed_admin_from_env(txn: &DatabaseTransaction) -> Result<usize, DomainError> {
    let emails = parse_admin_emails_from_env();
    for email in &emails {
        allowed_emails_sea::upsert_admin(txn, email)
            .await
            .map_err(crate::infra::db_errors::map_db_err)?;
    }
    Ok(emails.len())
}
