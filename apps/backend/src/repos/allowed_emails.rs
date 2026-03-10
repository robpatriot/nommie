//! Admission table repository for first-time login.

use sea_orm::{ConnectionTrait, DatabaseTransaction};
use unicode_normalization::UnicodeNormalization;

use crate::adapters::allowed_emails_sea;
use crate::errors::domain::DomainError;

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

/// Check if an email is admitted for first-time login.
///
/// - If the admission table is empty, all emails are admitted (open signup).
/// - If the table has rows, the email must match at least one pattern.
pub async fn is_email_admitted<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    email: &str,
) -> Result<bool, DomainError> {
    let rules = allowed_emails_sea::list_all(conn)
        .await
        .map_err(crate::infra::db_errors::map_db_err)?;
    if rules.is_empty() {
        return Ok(true);
    }

    let normalized = normalize(email);
    for rule in &rules {
        if matches_pattern(&normalized, &rule.pattern) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Check if there is an exact (non-wildcard) row for `normalized_email` with `is_admin=true`.
pub async fn is_exact_admin_match<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    normalized_email: &str,
) -> Result<bool, DomainError> {
    let rules = allowed_emails_sea::list_all(conn)
        .await
        .map_err(crate::infra::db_errors::map_db_err)?;
    for rule in &rules {
        if !rule.pattern.contains('*') && rule.pattern == normalized_email && rule.is_admin {
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
