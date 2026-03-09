use sea_orm::DatabaseTransaction;
use tracing::{info, trace, warn};
use unicode_normalization::UnicodeNormalization;

use crate::auth::google::VerifiedGoogleClaims;
use crate::errors::domain::{ConflictKind, DomainError, NotFoundKind, ValidationKind};
use crate::logging::pii::Redacted;
use crate::repos::users::{self as users_repo, User};
use crate::repos::{
    allowed_emails, auth_identities as auth_identities_repo, user_options as user_options_repo,
};

const PROVIDER_GOOGLE: &str = "google";

/// Redacts a provider user id for logging purposes.
/// Shows only the first 4 characters followed by asterisks.
fn redact_provider_user_id(user_id: &str) -> String {
    if user_id.len() <= 4 {
        "*".repeat(user_id.len())
    } else {
        format!("{}***", &user_id[..4])
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
    /// Accepts only verified claims from a server-verified Google ID token.
    /// This function is idempotent - calling it multiple times with the same
    /// provider_user_id will return the same user without creating duplicates.
    /// Returns the User that was found or created.
    pub async fn ensure_user(
        &self,
        txn: &DatabaseTransaction,
        claims: &VerifiedGoogleClaims,
    ) -> Result<User, DomainError> {
        let provider_user_id = &claims.sub;
        let email = &claims.email;
        let name = claims.name.as_deref();

        // Sanitize email: normalize (trim, lowercase, NFKC) and validate format
        let clean_email = sanitize_email(email)?;

        // Lookup by (provider, provider_user_id) first - fast path for repeat login
        let existing_identity =
            auth_identities_repo::find_by_provider_user_id(txn, PROVIDER_GOOGLE, provider_user_id)
                .await?;

        if let Some(identity) = existing_identity {
            return ensure_from_existing_identity(txn, email, identity).await;
        }

        // Not found by provider_user_id - check by email for mismatch
        let existing_by_email =
            auth_identities_repo::find_by_provider_email(txn, PROVIDER_GOOGLE, &clean_email)
                .await?;

        if let Some(identity) = existing_by_email {
            // Same email, different Google account - conflict
            warn!(
                user_id = identity.user_id,
                email = %Redacted(email),
                incoming_provider_user_id = %redact_provider_user_id(provider_user_id),
                existing_provider_user_id = %redact_provider_user_id(&identity.provider_user_id),
                "Google sub mismatch detected"
            );
            return Err(DomainError::conflict(
                ConflictKind::GoogleSubMismatch,
                "This email is already linked to a different Google account. Please use the original Google account or contact support.",
            ));
        }

        // New user - check admission table before creating
        if !allowed_emails::is_email_admitted(txn, &clean_email).await? {
            return Err(DomainError::validation(
                ValidationKind::EmailNotAllowed,
                "Access restricted. Please contact support if you believe this is an error."
                    .to_string(),
            ));
        }

        // Derive username from name or email local-part
        let username = derive_username(name, &clean_email);

        // Create user (no sub)
        let user =
            users_repo::create_user(txn, username.as_deref().unwrap_or("user"), false).await?;

        // Create identity - may race with concurrent insert, handle via ensure
        let (identity, inserted_identity) =
            auth_identities_repo::ensure_identity_by_provider_user_id(
                txn,
                user.id,
                PROVIDER_GOOGLE,
                provider_user_id,
                &clean_email,
            )
            .await?;

        // If another insert won the race (different user_id), we have a conflict
        if identity.user_id != user.id {
            users_repo::delete_user(txn, user.id).await?;
            return ensure_from_existing_identity(txn, email, identity).await;
        }

        if inserted_identity {
            info!(
                user_id = user.id,
                email = %Redacted(email),
                provider_user_id = %redact_provider_user_id(provider_user_id),
                "First user creation"
            );
        }

        user_options_repo::ensure_default_for_user(txn, user.id).await?;

        Ok(user)
    }
}

async fn ensure_from_existing_identity(
    txn: &DatabaseTransaction,
    email_for_logging: &str,
    identity: auth_identities_repo::AuthIdentity,
) -> Result<User, DomainError> {
    let user_id = identity.user_id;

    let mut updated_identity = identity.clone();
    updated_identity.last_login_at = Some(time::OffsetDateTime::now_utc());
    auth_identities_repo::update_identity(txn, updated_identity).await?;

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
