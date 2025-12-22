use sea_orm::DatabaseTransaction;
use tracing::{info, trace, warn};
use unicode_normalization::UnicodeNormalization;

use crate::config::email_allowlist::EmailAllowlist;
use crate::errors::domain::{ConflictKind, DomainError, NotFoundKind, ValidationKind};
use crate::logging::pii::Redacted;
use crate::repos::user_options as user_options_repo;
use crate::repos::users::{self as users_repo, User};

/// Redacts a google_sub value for logging purposes.
/// Shows only the first 4 characters followed by asterisks.
fn redact_google_sub(google_sub: &str) -> String {
    if google_sub.len() <= 4 {
        "*".repeat(google_sub.len())
    } else {
        format!("{}***", &google_sub[..4])
    }
}

/// Normalizes an email address for consistent storage and comparison.
///
/// Normalization includes:
/// - Trimming leading/trailing whitespace
/// - Converting to lowercase
/// - Applying Unicode NFKC normalization to handle visually equivalent but distinct codepoints
///
/// This ensures that logically identical emails (e.g., `EMAIL@Example.COM` and `email@example.com`,
/// or Unicode variants like `café@example.com` and `cafe\u{0301}@example.com`) normalize to the same value.
fn normalize_email(email: &str) -> String {
    email.trim().nfkc().collect::<String>().to_lowercase()
}

/// Validates that an email address has a reasonable format.
///
/// This is a lightweight validation that checks for:
/// - Exactly one '@' symbol
/// - Non-empty local part (before '@')
/// - Non-empty domain part (after '@')
/// - No leading or trailing '@' symbols
///
/// This validation is intentionally simple and permissive, as full RFC-compliant
/// email validation is complex. The goal is to catch obvious mistakes, not enforce
/// strict RFC 5322 compliance.
fn validate_email(email: &str) -> Result<(), DomainError> {
    // Find the position of '@'
    let at_pos = match email.find('@') {
        Some(pos) => pos,
        None => {
            return Err(DomainError::validation(
                ValidationKind::InvalidEmail,
                "Email must contain an '@' symbol",
            ))
        }
    };

    // Check for exactly one '@' symbol (look for another '@' after the first)
    if email[at_pos + 1..].contains('@') {
        return Err(DomainError::validation(
            ValidationKind::InvalidEmail,
            "Email must contain exactly one '@' symbol",
        ));
    }

    // Check that local part (before '@') is non-empty
    if at_pos == 0 {
        return Err(DomainError::validation(
            ValidationKind::InvalidEmail,
            "Email local part (before '@') cannot be empty",
        ));
    }

    // Check that domain part (after '@') is non-empty
    if at_pos == email.len() - 1 {
        return Err(DomainError::validation(
            ValidationKind::InvalidEmail,
            "Email domain part (after '@') cannot be empty",
        ));
    }

    Ok(())
}

/// Normalizes and validates an email address.
///
/// This function performs both normalization (trimming, lowercasing, NFKC) and
/// lightweight validation to ensure the email is in a reasonable format.
///
/// Returns the normalized email string if valid, or an error if validation fails.
fn sanitize_email(email: &str) -> Result<String, DomainError> {
    let normalized = normalize_email(email);
    validate_email(&normalized)?;
    Ok(normalized)
}

/// User domain service.
#[derive(Default)]
pub struct UserService;

impl UserService {
    /// Ensures a user exists for Google OAuth, creating one if necessary.
    /// This function is idempotent - calling it multiple times with the same email
    /// will return the same user without creating duplicates.
    /// Returns the User that was found or created.
    pub async fn ensure_user(
        &self,
        txn: &DatabaseTransaction,
        email: &str,
        name: Option<&str>,
        google_sub: &str,
        email_allowlist: Option<&EmailAllowlist>,
    ) -> Result<User, DomainError> {
        // Sanitize email: normalize (trim, lowercase, NFKC) and validate format
        let clean_email = sanitize_email(email)?;

        // Look up existing user credentials by email
        let existing_credential = users_repo::find_credentials_by_email(txn, &clean_email).await?;

        if let Some(credential) = existing_credential {
            return ensure_from_existing_credential(txn, email, google_sub, credential).await;
        }

        // User doesn't exist, create new user and credentials

        // Check email allowlist before creating new user (defense in depth)
        if let Some(allowlist) = email_allowlist {
            if !allowlist.is_allowed(&clean_email) {
                return Err(DomainError::validation(
                    ValidationKind::InvalidEmail,
                    "Access restricted. Please contact support if you believe this is an error."
                        .to_string(),
                ));
            }
        }

        // Derive username from name or email local-part
        let username = derive_username(name, &clean_email);

        // Ensure the user exists for this google_sub without aborting the transaction
        // on concurrent inserts.
        let (user, created_user) = users_repo::ensure_user_by_sub(
            txn,
            google_sub,
            username.as_deref().unwrap_or("user"),
            false,
        )
        .await?;

        // Ensure credentials exist for this email without aborting the transaction on
        // concurrent inserts. If the email is already owned by a different user, delete
        // the newly created user (if any) to avoid committing an orphan row.
        let (credential, inserted_credential) =
            users_repo::ensure_credentials_by_email(txn, user.id, &clean_email, Some(google_sub))
                .await?;

        if credential.user_id != user.id {
            if created_user {
                users_repo::delete_user(txn, user.id).await?;
            }
            return ensure_from_existing_credential(txn, email, google_sub, credential).await;
        }

        if created_user && inserted_credential {
            info!(
                user_id = user.id,
                email = %Redacted(email),
                google_sub = %redact_google_sub(google_sub),
                "First user creation"
            );
        }

        ensure_from_existing_credential(txn, email, google_sub, credential).await
    }
}

async fn ensure_from_existing_credential(
    txn: &DatabaseTransaction,
    email_for_logging: &str,
    google_sub: &str,
    credential: crate::repos::users::UserCredentials,
) -> Result<User, DomainError> {
    if let Some(existing_google_sub) = &credential.google_sub {
        if existing_google_sub != google_sub {
            warn!(
                user_id = credential.user_id,
                email = %Redacted(email_for_logging),
                incoming_google_sub = %redact_google_sub(google_sub),
                existing_google_sub = %redact_google_sub(existing_google_sub),
                "Google sub mismatch detected"
            );
            return Err(DomainError::conflict(
                ConflictKind::GoogleSubMismatch,
                "This email is already linked to a different Google account. Please use the original Google account or contact support.",
            ));
        }
    }

    let user_id = credential.user_id;
    let mut updated_credential = credential.clone();
    updated_credential.last_login = Some(time::OffsetDateTime::now_utc());

    if updated_credential.google_sub.is_none() {
        info!(
            user_id = user_id,
            email = %Redacted(email_for_logging),
            google_sub = %redact_google_sub(google_sub),
            "Setting google_sub for existing user (was previously NULL)"
        );
        updated_credential.google_sub = Some(google_sub.to_string());
    }

    updated_credential.updated_at = time::OffsetDateTime::now_utc();
    users_repo::update_credentials(txn, updated_credential).await?;

    let user = users_repo::find_user_by_id(txn, user_id)
        .await?
        .ok_or_else(|| DomainError::not_found(NotFoundKind::User, "User not found"))?;

    user_options_repo::ensure_default_for_user(txn, user.id).await?;

    trace!(
        user_id = user_id,
        email = %Redacted(email_for_logging),
        "Repeat login for existing user"
    );

    Ok(user)
}

/// Derives a username from the provided name or email local-part.
/// Returns None if no suitable username can be derived.
fn derive_username(name: Option<&str>, email: &str) -> Option<String> {
    if let Some(name) = name {
        // Use the provided name, cleaned up
        let clean_name = name.trim();
        if !clean_name.is_empty() {
            return Some(clean_name.to_string());
        }
    }

    // Fall back to email local-part (before @)
    if let Some(at_pos) = email.find('@') {
        let local_part = &email[..at_pos];
        if !local_part.is_empty() {
            return Some(local_part.to_string());
        }
    }

    // No suitable username found
    None
}
