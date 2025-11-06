use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpMessage, HttpRequest};
use sea_orm::TransactionTrait;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::auth::claims::BackendClaims;
use crate::db::require_db;
use crate::db::txn::SharedTxn;
use crate::error::AppError;
use crate::repos::users;
use crate::state::app_state::AppState;

/// Current user record from the database
/// This contains the actual user data from the database, extracted from JWT claims
/// stored in request extensions by the JwtExtract middleware
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurrentUser {
    pub id: i64,
    pub sub: String,
    pub email: Option<String>,
}

impl FromRequest for CurrentUser {
    type Error = AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // [AUTH_BYPASS] START - Temporary debugging feature - remove when done
            // Check if authentication bypass is enabled via environment variable
            let disable_auth = std::env::var("DISABLE_AUTH")
                .unwrap_or_default()
                .parse::<bool>()
                .unwrap_or(false);

            if disable_auth {
                warn!("⚠️  AUTHENTICATION BYPASSED - This should only be enabled for debugging!");

                // Get test user configuration from environment
                let test_sub =
                    std::env::var("TEST_USER_SUB").unwrap_or_else(|_| "test-user".to_string());
                let test_email = std::env::var("TEST_USER_EMAIL")
                    .unwrap_or_else(|_| "test@example.com".to_string());

                // Get database connection from AppState
                let app_state = req.app_data::<web::Data<AppState>>().ok_or_else(|| {
                    AppError::internal(
                        crate::errors::ErrorCode::InternalError,
                        "AppState not available".to_string(),
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "AppState missing from request",
                        ),
                    )
                })?;

                // Try to find existing test user
                let user = if let Some(shared_txn) = SharedTxn::from_req(&req) {
                    users::find_user_by_sub(shared_txn.transaction(), &test_sub).await?
                } else {
                    let db = require_db(app_state)?;
                    users::find_user_by_sub(db, &test_sub).await?
                };

                // If user doesn't exist, create it
                let user = match user {
                    Some(u) => u,
                    None => {
                        // Create test user in a transaction
                        let db = require_db(app_state)?;
                        let txn = db.begin().await.map_err(|e| {
                            AppError::db("failed to begin transaction for test user creation", e)
                        })?;

                        // Create user
                        let new_user = users::create_user(&txn, &test_sub, "Test User", false)
                            .await
                            .map_err(AppError::from)?;

                        // Create credentials
                        users::create_credentials(&txn, new_user.id, &test_email, Some(&test_sub))
                            .await
                            .map_err(AppError::from)?;

                        // Commit transaction
                        txn.commit()
                            .await
                            .map_err(|e| AppError::db("failed to commit test user creation", e))?;

                        // Fetch the created user
                        users::find_user_by_sub(db, &test_sub)
                            .await?
                            .ok_or_else(|| {
                                AppError::internal(
                                    crate::errors::ErrorCode::InternalError,
                                    "Failed to find newly created test user".to_string(),
                                    std::io::Error::new(
                                        std::io::ErrorKind::NotFound,
                                        "Test user not found after creation",
                                    ),
                                )
                            })?
                    }
                };

                return Ok(CurrentUser {
                    id: user.id,
                    sub: user.sub,
                    email: Some(test_email),
                });
            }
            // [AUTH_BYPASS] END

            // Read BackendClaims from request extensions (stored by JwtExtract middleware)
            let claims = req
                .extensions()
                .get::<BackendClaims>()
                .ok_or_else(AppError::unauthorized_missing_bearer)?
                .clone();

            // Get database connection from AppState
            let app_state = req.app_data::<web::Data<AppState>>().ok_or_else(|| {
                AppError::internal(
                    crate::errors::ErrorCode::InternalError,
                    "AppState not available".to_string(),
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "AppState missing from request",
                    ),
                )
            })?;

            // Look up user by sub in database
            let user = if let Some(shared_txn) = SharedTxn::from_req(&req) {
                // Use shared transaction if present
                users::find_user_by_sub(shared_txn.transaction(), &claims.sub).await?
            } else {
                // Fall back to pooled connection
                let db = require_db(app_state)?;
                users::find_user_by_sub(db, &claims.sub).await?
            };

            let user = user.ok_or(AppError::forbidden_user_not_found())?;

            // Use sub and email from JWT claims (already validated) rather than from database
            // We still need user.id from the database lookup
            Ok(CurrentUser {
                id: user.id,
                sub: claims.sub,
                email: Some(claims.email),
            })
        })
    }
}
