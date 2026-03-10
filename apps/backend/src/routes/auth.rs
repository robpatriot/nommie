use std::time::SystemTime;

use actix_web::{web, HttpRequest, HttpResponse, Result};
use serde::{Deserialize, Serialize};

use crate::auth::jwt::{mint_access_token, verify_access_token};
use crate::db::require_db;
use crate::db::txn::with_txn;
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::extractors::ValidatedJson;
use crate::services::users::UserService;
use crate::state::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub id_token: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckAllowlistRequest {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct CheckAllowlistResponse {
    pub allowed: bool,
}

/// Handle Google OAuth login with verified ID token.
/// Verifies the token server-side, extracts trusted claims, and returns our JWT.
async fn login(
    http_req: HttpRequest,
    req: ValidatedJson<LoginRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let id_token = req.id_token.trim();
    if id_token.is_empty() {
        return Err(AppError::bad_request(
            ErrorCode::InvalidIdToken,
            "id_token cannot be empty".to_string(),
        ));
    }

    let claims = app_state
        .config
        .google_verifier
        .verify(id_token)
        .await
        .inspect_err(|_| crate::logging::security::login_failed("invalid_id_token", None))?;

    let verified_email = claims.email.clone();

    let user = match with_txn(Some(&http_req), &app_state, |txn| {
        let c = claims.clone();
        Box::pin(async move {
            let service = UserService;
            Ok(service.ensure_user(txn, &c).await?)
        })
    })
    .await
    {
        Ok(u) => u,
        Err(e) if e.code() == ErrorCode::UniqueEmail => {
            // Concurrent insert won; retry with fresh transaction (winner will have committed)
            with_txn(Some(&http_req), &app_state, |txn| {
                let c = claims.clone();
                Box::pin(async move {
                    let service = UserService;
                    Ok(service.ensure_user(txn, &c).await?)
                })
            })
            .await?
        }
        Err(e) => return Err(e),
    };

    let token = mint_access_token(
        &user.id.to_string(),
        &verified_email,
        SystemTime::now(),
        app_state.security(),
    )?;

    let response = LoginResponse { token };
    Ok(HttpResponse::Ok().json(response))
}

/// Refresh backend JWT. Requires Bearer token with current valid backend JWT.
/// Returns a new JWT with extended expiry.
async fn refresh(
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let auth_header = req
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(AppError::unauthorized_missing_bearer)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))
        .ok_or_else(AppError::unauthorized_missing_bearer)?;

    let claims = verify_access_token(token.trim(), app_state.security())?;

    let new_token = mint_access_token(
        &claims.sub,
        &claims.email,
        SystemTime::now(),
        app_state.security(),
    )?;

    let response = LoginResponse { token: new_token };
    Ok(HttpResponse::Ok().json(response))
}

/// Lightweight endpoint to check if an email is admitted for first-time login.
///
/// Prevents unnecessary backend API calls and session creation for non-admitted emails.
/// No authentication required - this is a public endpoint.
///
/// Returns allowed=true if:
/// - The email has an existing linked identity (repeat login), or
/// - The email matches the admission table (first-time login).
///
/// Existing users are always allowed regardless of the admission table.
async fn check_allowlist(
    req: ValidatedJson<CheckAllowlistRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    if req.email.trim().is_empty() {
        return Err(AppError::bad_request(
            ErrorCode::InvalidEmail,
            "Email cannot be empty".to_string(),
        ));
    }

    let db = require_db(app_state.as_ref())?;
    let email = crate::repos::allowed_emails::normalize(&req.email);

    // Existing linked users bypass admission check
    let existing = crate::repos::auth_identities::find_by_provider_email(&db, "google", &email)
        .await
        .map_err(AppError::from)?;

    if existing.is_some() {
        let response = CheckAllowlistResponse { allowed: true };
        return Ok(HttpResponse::Ok().json(response));
    }

    // First-time login: must be in admission table
    let allowed = crate::repos::allowed_emails::is_email_admitted(&db, &email)
        .await
        .map_err(AppError::from)?;

    if !allowed {
        return Err(AppError::email_not_allowed());
    }

    let response = CheckAllowlistResponse { allowed: true };
    Ok(HttpResponse::Ok().json(response))
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/login").route(web::post().to(login)));
    cfg.service(web::resource("/refresh").route(web::post().to(refresh)));
    cfg.service(web::resource("/check-allowlist").route(web::post().to(check_allowlist)));
}
