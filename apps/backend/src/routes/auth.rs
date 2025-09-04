use std::time::SystemTime;

use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};

use crate::auth::jwt::mint_access_token;
use crate::error::AppError;
use crate::services::users::ensure_user;
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
    req: web::Json<LoginRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    // Validate required fields
    if req.email.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_EMAIL",
            "Email cannot be empty".to_string(),
        ));
    }

    if req.google_sub.trim().is_empty() {
        return Err(AppError::bad_request(
            "INVALID_GOOGLE_SUB",
            "Google sub cannot be empty".to_string(),
        ));
    }

    let db = app_state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("Database connection not available".to_string()))?;

    let (user, email) = ensure_user(&req.email, req.name.as_deref(), &req.google_sub, db).await?;

    let token = mint_access_token(&user.sub, &email, SystemTime::now(), &app_state.security)?;

    let response = LoginResponse { token };
    Ok(HttpResponse::Ok().json(response))
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/api/auth/login").route(web::post().to(login)));
}
