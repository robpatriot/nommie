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

    // Check email allowlist before proceeding (prevents both signup and login)
    // The allowlist's is_allowed() method handles normalization internally
    if let Some(allowlist) = &app_state.email_allowlist {
        if !allowlist.is_allowed(&req.email) {
            return Err(AppError::email_not_allowed());
        }
    }

    // Prepare owned values to move into the txn closure
    let email = req.email.clone();
    let name = req.name.clone();
    let google_sub = req.google_sub.clone();
    let email_allowlist = app_state.email_allowlist.clone();

    // Own the transaction boundary here and pass a borrowed txn to the service
    let user = with_txn(Some(&http_req), &app_state, |txn| {
        // Box the async block so its lifetime is tied to `txn` (no 'static)
        Box::pin(async move {
            let service = UserService;
            Ok(service
                .ensure_user(
                    txn,
                    &email,
                    name.as_deref(),
                    &google_sub,
                    email_allowlist.as_ref(),
                )
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
    // Route path is relative to the scope in main.rs (/api/auth)
    cfg.service(web::resource("/login").route(web::post().to(login)));
}
