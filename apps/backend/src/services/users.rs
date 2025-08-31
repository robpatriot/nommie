use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use uuid::Uuid;

use crate::{
    entities::{user_credentials, users, User},
    error::AppError,
};

/// Ensures a user exists for Google OAuth, creating one if necessary.
/// This function is idempotent - calling it multiple times with the same email
/// will return the same user without creating duplicates.
pub async fn ensure_user(
    email: &str,
    name: Option<&str>,
    google_sub: &str,
    db: &DatabaseConnection,
) -> Result<User, AppError> {
    // Start a transaction to ensure data consistency
    let txn = db
        .begin()
        .await
        .map_err(|e| AppError::db(format!("Failed to begin transaction: {e}")))?;

    // Look up existing user credentials by email
    let existing_credential = user_credentials::Entity::find()
        .filter(user_credentials::Column::Email.eq(email))
        .one(&txn)
        .await
        .map_err(|e| AppError::db(format!("Failed to query user credentials: {e}")))?;

    match existing_credential {
        Some(credential) => {
            // User exists, update last_login and google_sub if needed
            let user_id = credential.user_id;
            let mut credential_active: user_credentials::ActiveModel = credential.clone().into();
            credential_active.last_login = Set(Some(get_current_time()));

            // Only update google_sub if it's currently NULL
            if credential.google_sub.is_none() {
                credential_active.google_sub = Set(Some(google_sub.to_string()));
            }

            credential_active.updated_at = Set(get_current_time());

            credential_active
                .update(&txn)
                .await
                .map_err(|e| AppError::db(format!("Failed to update user credentials: {e}")))?;

            // Fetch and return the linked user
            let user = users::Entity::find_by_id(user_id)
                .one(&txn)
                .await
                .map_err(|e| AppError::db(format!("Failed to fetch user: {e}")))?
                .ok_or_else(|| {
                    AppError::internal("User not found after updating credentials".to_string())
                })?;

            txn.commit()
                .await
                .map_err(|e| AppError::db(format!("Failed to commit transaction: {e}")))?;

            Ok(user)
        }
        None => {
            // User doesn't exist, create new user and credentials
            let user_id = Uuid::new_v4();
            let now = get_current_time();

            // Derive username from name or email local-part
            let username = derive_username(name, email);

            // Create new user
            let user_active = users::ActiveModel {
                id: Set(user_id),
                username: Set(username),
                is_ai: Set(false),
                created_at: Set(now),
                updated_at: Set(now),
            };

            let user = user_active
                .insert(&txn)
                .await
                .map_err(|e| AppError::db(format!("Failed to create user: {e}")))?;

            // Create new user credentials
            let credential_active = user_credentials::ActiveModel {
                id: Set(Uuid::new_v4()),
                user_id: Set(user_id),
                password_hash: Set(None),
                email: Set(email.to_string()),
                google_sub: Set(Some(google_sub.to_string())),
                last_login: Set(Some(now)),
                created_at: Set(now),
                updated_at: Set(now),
            };

            credential_active
                .insert(&txn)
                .await
                .map_err(|e| AppError::db(format!("Failed to create user credentials: {e}")))?;

            txn.commit()
                .await
                .map_err(|e| AppError::db(format!("Failed to commit transaction: {e}")))?;

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
