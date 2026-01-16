use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpMessage, HttpRequest};
use serde::{Deserialize, Serialize};

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
            let user = match SharedTxn::from_req(&req) {
                Some(shared_txn) => {
                    // Use shared transaction if present
                    users::find_user_by_sub(shared_txn.transaction(), &claims.sub).await?
                }
                _ => {
                    // Fall back to pooled connection
                    let db = require_db(app_state)?;
                    users::find_user_by_sub(db, &claims.sub).await?
                }
            };

            let user = user.ok_or(AppError::unauthorized())?;

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
