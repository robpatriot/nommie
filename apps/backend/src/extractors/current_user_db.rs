use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpRequest};
use serde::{Deserialize, Serialize};

use super::current_user::CurrentUser;
use crate::db::require_db;
use crate::db::txn::SharedTxn;
use crate::error::AppError;
use crate::repos::users;
use crate::state::app_state::AppState;

/// Database-backed current user record
/// This contains the actual user data from the database
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurrentUserRecord {
    pub id: i64,
    pub sub: String,
    pub email: Option<String>,
}

impl FromRequest for CurrentUserRecord {
    type Error = AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();

        Box::pin(async move {
            // First extract CurrentUser (claims) to enforce H1 authentication
            let current_user = CurrentUser::from_request(&req, &mut payload).await?;

            // Get database connection from AppState
            let app_state = req
                .app_data::<web::Data<AppState>>()
                .ok_or_else(|| AppError::internal("AppState not available"))?;

            // Look up user by sub in database
            let user = if let Some(shared_txn) = SharedTxn::from_req(&req) {
                // Use shared transaction if present
                users::find_user_by_sub(shared_txn.transaction(), &current_user.sub).await?
            } else {
                // Fall back to pooled connection
                let db = require_db(app_state)?;
                users::find_user_by_sub(db, &current_user.sub).await?
            };

            let user = user.ok_or(AppError::forbidden_user_not_found())?;

            // For now, we'll return None for email since it's not in the users table
            // In a real implementation, you might want to join with user_credentials
            Ok(CurrentUserRecord {
                id: user.id,
                sub: user.sub,
                email: None, // users table doesn't have email field
            })
        })
    }
}
