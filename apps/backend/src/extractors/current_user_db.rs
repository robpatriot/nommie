use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpRequest};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use super::current_user::CurrentUser;
use crate::entities::users;
use crate::error::AppError;
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
            let db = app_state
                .db()
                .ok_or_else(|| AppError::db_unavailable("Database unavailable"))?;

            // Look up user by sub in database
            let user = users::Entity::find()
                .filter(users::Column::Sub.eq(&current_user.sub))
                .one(db)
                .await
                .map_err(|e| AppError::db(format!("Failed to query user by sub: {e}")))?;

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
