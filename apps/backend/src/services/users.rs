use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, NotSet, QueryFilter, Set,
};
use tracing::{debug, info, warn};
use users::Model as User;

use crate::entities::{user_credentials, users};
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::infra::db_errors::map_db_err;
use crate::logging::pii::Redacted;

/// Redacts a google_sub value for logging purposes.
/// Shows only the first 4 characters followed by asterisks.
fn redact_google_sub(google_sub: &str) -> String {
    if google_sub.len() <= 4 {
        "*".repeat(google_sub.len())
    } else {
        format!("{}***", &google_sub[..4])
    }
}

/// Ensures a user exists for Google OAuth, creating one if necessary.
/// This function is idempotent - calling it multiple times with the same email
/// will return the same user without creating duplicates.
/// Returns the User that was found or created.
pub async fn ensure_user(
    email: String,
    name: Option<String>,
    google_sub: String,
    conn: &impl ConnectionTrait,
) -> Result<User, AppError> {
    // Look up existing user credentials by email
    let existing_credential = user_credentials::Entity::find()
        .filter(user_credentials::Column::Email.eq(&email))
        .one(conn)
        .await
        .map_err(|e| AppError::from(map_db_err(e)))?;

    match existing_credential {
        Some(credential) => {
            // User exists, check for google_sub mismatch
            if let Some(existing_google_sub) = &credential.google_sub {
                if existing_google_sub != &google_sub {
                    warn!(
                        user_id = credential.user_id,
                        email = %Redacted(&email),
                        incoming_google_sub = %redact_google_sub(&google_sub),
                        existing_google_sub = %redact_google_sub(existing_google_sub),
                        "Google sub mismatch detected"
                    );
                    return Err(AppError::conflict(
                        ErrorCode::GoogleSubMismatch,
                        "This email is already linked to a different Google account. Please use the original Google account or contact support.".to_string(),
                    ));
                }
            }

            // User exists, update last_login and google_sub if needed
            let user_id = credential.user_id;
            let mut credential_active: user_credentials::ActiveModel = credential.clone().into();
            credential_active.last_login = Set(Some(get_current_time()));

            // Only update google_sub if it's currently NULL
            if credential.google_sub.is_none() {
                info!(
                    user_id = user_id,
                    email = %Redacted(&email),
                    google_sub = %redact_google_sub(&google_sub),
                    "Setting google_sub for existing user (was previously NULL)"
                );
                credential_active.google_sub = Set(Some(google_sub.clone()));
            }

            credential_active.updated_at = Set(get_current_time());

            credential_active
                .update(conn)
                .await
                .map_err(|e| AppError::from(map_db_err(e)))?;

            // Fetch and return the linked user
            let user = users::Entity::find_by_id(user_id)
                .one(conn)
                .await
                .map_err(|e| AppError::from(map_db_err(e)))?
                .ok_or_else(|| AppError::not_found(ErrorCode::UserNotFound, "User not found"))?;

            // Log repeat login (same email + same google_sub)
            debug!(
                user_id = user_id,
                email = %Redacted(&email),
                "Repeat login for existing user"
            );

            Ok(user)
        }
        None => {
            // User doesn't exist, create new user and credentials
            let now = get_current_time();

            // Derive username from name or email local-part
            let username = derive_username(name.as_deref(), &email);

            // Create new user with auto-generated ID and sub from google_sub
            let sub_for_user = google_sub.clone(); // clone once; original is used for credentials below
            let user_active = users::ActiveModel {
                id: NotSet,             // Let database auto-generate
                sub: Set(sub_for_user), // Use google_sub as the external identifier
                username: Set(username),
                is_ai: Set(false),
                created_at: Set(now),
                updated_at: Set(now),
            };

            let user = user_active
                .insert(conn)
                .await
                .map_err(|e| AppError::from(map_db_err(e)))?;

            // Create new user credentials with auto-generated ID
            let credential_active = user_credentials::ActiveModel {
                id: NotSet,            // Let database auto-generate
                user_id: Set(user.id), // Use the ID from the created user
                password_hash: Set(None),
                email: Set(email.clone()),
                google_sub: Set(Some(google_sub.clone())), // clone here to keep original for logging
                last_login: Set(Some(now)),
                created_at: Set(now),
                updated_at: Set(now),
            };

            credential_active
                .insert(conn)
                .await
                .map_err(|e| AppError::from(map_db_err(e)))?;

            // Log first user creation
            info!(
                user_id = user.id,
                email = %Redacted(&email),
                google_sub = %redact_google_sub(&google_sub),
                "First user creation"
            );

            Ok(user)
        }
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
