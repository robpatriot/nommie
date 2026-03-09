//! Admission table repository for first-time login.

use sea_orm::{ConnectionTrait, DatabaseTransaction};
use unicode_normalization::UnicodeNormalization;

use crate::adapters::allowed_emails_sea;
use crate::errors::domain::DomainError;

/// Normalize an email/pattern for consistent storage and lookup.
fn normalize(value: &str) -> String {
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

/// Check if an email is admitted for first-time login.
///
/// - If the admission table is empty, all emails are admitted (open signup).
/// - If the table has rows, the email must match at least one pattern.
pub async fn is_email_admitted<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    email: &str,
) -> Result<bool, DomainError> {
    let patterns = allowed_emails_sea::list_all(conn)
        .await
        .map_err(crate::infra::db_errors::map_db_err)?;
    if patterns.is_empty() {
        return Ok(true);
    }

    let normalized = normalize(email);
    for pattern in &patterns {
        if matches_pattern(&normalized, pattern) {
            return Ok(true);
        }
    }
    Ok(false)
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

/// Seed missing entries from ALLOWED_EMAILS into the admission table.
/// Additive and idempotent.
pub async fn seed_from_env(txn: &DatabaseTransaction) -> Result<usize, DomainError> {
    let patterns = parse_allowed_emails_from_env();
    let mut inserted = 0;
    for pattern in patterns {
        if allowed_emails_sea::insert_if_not_exists(txn, &pattern)
            .await
            .map_err(crate::infra::db_errors::map_db_err)?
        {
            inserted += 1;
        }
    }
    Ok(inserted)
}
