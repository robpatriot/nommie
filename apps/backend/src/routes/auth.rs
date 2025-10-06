use std::time::SystemTime;

use actix_web::{web, HttpRequest, HttpResponse, Result};
use serde::{Deserialize, Serialize};

use crate::auth::jwt::mint_access_token;
use crate::db::txn::with_txn;
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::extractors::ValidatedJson;
use crate::services::users::UserService;
use crate::state::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    #[serde(default)]
    pub email: String,
    pub name: Option<String>,
    #[serde(default)]
    pub google_sub: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
}

/// Handle Google OAuth login callback
/// Creates or reuses a user based on email and returns a JWT token
async fn login(
    http_req: HttpRequest,
    req: ValidatedJson<LoginRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    // Validate required fields
    if req.email.trim().is_empty() {
        return Err(AppError::bad_request(
            ErrorCode::InvalidEmail,
            "Email cannot be empty".to_string(),
        ));
    }

    if req.google_sub.trim().is_empty() {
        return Err(AppError::bad_request(
            ErrorCode::InvalidGoogleSub,
            "Google sub cannot be empty".to_string(),
        ));
    }

    // Prepare owned values to move into the txn closure
    let email = req.email.clone();
    let name = req.name.clone();
    let google_sub = req.google_sub.clone();

    // Own the transaction boundary here and pass a borrowed txn to the service
    let user = with_txn(Some(&http_req), &app_state, |txn| {
        // Box the async block so its lifetime is tied to `txn` (no 'static)
        Box::pin(async move {
            let service = UserService::new();
            Ok(service
                .ensure_user(txn, &email, name.as_deref(), &google_sub)
                .await?)
        })
    })
    .await?;

    let token = mint_access_token(
        &user.sub,
        &req.email,
        SystemTime::now(),
        &app_state.security,
    )?;

    let response = LoginResponse { token };
    Ok(HttpResponse::Ok().json(response))
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/api/auth/login").route(web::post().to(login)));
}
