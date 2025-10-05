use tracing::{debug, info, warn};

use crate::db::DbConn;
use crate::errors::domain::{ConflictKind, DomainError, NotFoundKind};
use crate::logging::pii::Redacted;
use crate::repos::users::{User, UserRepo};

/// Redacts a google_sub value for logging purposes.
/// Shows only the first 4 characters followed by asterisks.
fn redact_google_sub(google_sub: &str) -> String {
    if google_sub.len() <= 4 {
        "*".repeat(google_sub.len())
    } else {
        format!("{}***", &google_sub[..4])
    }
}

/// User domain service.
pub struct UserService;

impl UserService {
    pub fn new() -> Self {
        Self
    }

    /// Ensures a user exists for Google OAuth, creating one if necessary.
    /// This function is idempotent - calling it multiple times with the same email
    /// will return the same user without creating duplicates.
    /// Returns the User that was found or created.
    pub async fn ensure_user(
        &self,
        repo: &dyn UserRepo,
        conn: &dyn DbConn,
        email: &str,
        name: Option<&str>,
        google_sub: &str,
    ) -> Result<User, DomainError> {
        // Look up existing user credentials by email
        let existing_credential = repo.find_credentials_by_email(conn, email).await?;

        match existing_credential {
            Some(credential) => {
                // User exists, check for google_sub mismatch
                if let Some(existing_google_sub) = &credential.google_sub {
                    if existing_google_sub != google_sub {
                        warn!(
                            user_id = credential.user_id,
                            email = %Redacted(email),
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

                // User exists, update last_login and google_sub if needed
                let user_id = credential.user_id;
                let mut updated_credential = credential.clone();
                updated_credential.last_login = Some(get_current_time());

                // Only update google_sub if it's currently NULL
                if updated_credential.google_sub.is_none() {
                    info!(
                        user_id = user_id,
                        email = %Redacted(email),
                        google_sub = %redact_google_sub(google_sub),
                        "Setting google_sub for existing user (was previously NULL)"
                    );
                    updated_credential.google_sub = Some(google_sub.to_string());
                }

                updated_credential.updated_at = get_current_time();

                repo.update_credentials(conn, updated_credential).await?;

                // Fetch and return the linked user
                let user = repo
                    .find_user_by_id(conn, user_id)
                    .await?
                    .ok_or_else(|| DomainError::not_found(NotFoundKind::User, "User not found"))?;

                // Log repeat login (same email + same google_sub)
                debug!(
                    user_id = user_id,
                    email = %Redacted(email),
                    "Repeat login for existing user"
                );

                Ok(user)
            }
            None => {
                // User doesn't exist, create new user and credentials

                // Derive username from name or email local-part
                let username = derive_username(name, email);

                // Create new user with auto-generated ID and sub from google_sub
                let user = repo
                    .create_user(
                        conn,
                        google_sub,
                        username.as_deref().unwrap_or("user"),
                        false,
                    )
                    .await?;

                // Create new user credentials with auto-generated ID
                repo.create_credentials(conn, user.id, email, Some(google_sub))
                    .await?;

                // Log first user creation
                info!(
                    user_id = user.id,
                    email = %Redacted(email),
                    google_sub = %redact_google_sub(google_sub),
                    "First user creation"
                );

                Ok(user)
            }
        }
    }
}

impl Default for UserService {
    fn default() -> Self {
        Self::new()
    }
}

/// Gets the current UTC time as an OffsetDateTime
fn get_current_time() -> time::OffsetDateTime {
    time::OffsetDateTime::now_utc()
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
