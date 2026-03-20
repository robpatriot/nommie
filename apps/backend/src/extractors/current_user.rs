use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpMessage, HttpRequest};
use serde::{Deserialize, Serialize};

use crate::auth::session::SessionData;
use crate::db::require_db;
use crate::db::txn::SharedTxn;
use crate::entities::users::UserRole;
use crate::error::AppError;
use crate::repos::users;
use crate::state::app_state::AppState;

/// Current user record from the database
/// This contains the actual user data from the database, extracted from SessionData
/// stored in request extensions by the SessionExtract middleware
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurrentUser {
    pub id: i64,
    pub email: Option<String>,
    pub username: Option<String>,
    pub role: UserRole,
    pub sub: String,
}

impl FromRequest for CurrentUser {
    type Error = AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // Read SessionData from request extensions (stored by SessionExtract middleware)
            let session_data = req
                .extensions()
                .get::<SessionData>()
                .ok_or_else(AppError::unauthorized)?
                .clone();

            let user_id = session_data.user_id;

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

            // Look up user by id in database
            let user = match SharedTxn::from_req(&req) {
                Some(shared_txn) => {
                    users::find_user_by_id(shared_txn.transaction(), user_id).await?
                }
                _ => {
                    let db = require_db(app_state)?;
                    users::find_user_by_id(&db, user_id).await?
                }
            };

            let user = user.ok_or(AppError::unauthorized())?;

            Ok(CurrentUser {
                id: user.id,
                email: Some(session_data.email),
                username: user.username,
                role: user.role,
                sub: session_data.sub,
            })
        })
    }
}
