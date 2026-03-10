use std::time::SystemTime;

use actix_web::{web, HttpRequest, HttpResponse, Result};
use serde::{Deserialize, Serialize};

use crate::auth::jwt::{mint_access_token, verify_access_token};
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

#[derive(Clone, Debug, Deserialize)]
pub struct CheckAllowlistRequest {
    pub email: String,
    /// Google OAuth `sub` (provider_user_id). When present, returning users are
    /// recognized by identity even if email has changed.
    #[serde(default)]
    pub sub: Option<String>,
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
    let admission_mode = app_state.config.admission_mode;

    let user = match with_txn(Some(&http_req), &app_state, |txn| {
        let c = claims.clone();
        Box::pin(async move {
            let service = UserService;
            Ok(service.ensure_user(txn, &c, admission_mode).await?)
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
                    Ok(service.ensure_user(txn, &c, admission_mode).await?)
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
/// - An existing identity is found by (provider, provider_user_id) when `sub` is provided, or
/// - The email has an existing linked identity (repeat login), or
/// - The email matches the admission table (first-time login).
///
/// When `sub` is present, we check by provider_user_id first (aligned with ensure_user),
/// so returning users with changed email are recognized.
async fn check_allowlist(
    http_req: HttpRequest,
    req: ValidatedJson<CheckAllowlistRequest>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    if req.email.trim().is_empty() {
        return Err(AppError::bad_request(
            ErrorCode::InvalidEmail,
            "Email cannot be empty".to_string(),
        ));
    }

    let email = crate::repos::allowed_emails::normalize(&req.email);
    let admission_mode = app_state.config.admission_mode;

    let result = with_txn(Some(&http_req), &app_state, |txn| {
        let req = req.clone();
        Box::pin(async move {
            // When sub is provided, check by (provider, provider_user_id) first - same as ensure_user
            if let Some(ref sub) = req.sub {
                let sub_trimmed = sub.trim();
                if !sub_trimmed.is_empty() {
                    let existing = crate::repos::auth_identities::find_by_provider_user_id(
                        txn,
                        "google",
                        sub_trimmed,
                    )
                    .await
                    .map_err(AppError::from)?;

                    if existing.is_some() {
                        return Ok(CheckAllowlistResponse { allowed: true });
                    }
                }
            }

            // Fall back to email-based check for returning users without sub or not found by sub
            let existing =
                crate::repos::auth_identities::find_by_provider_email(txn, "google", &email)
                    .await
                    .map_err(AppError::from)?;

            if existing.is_some() {
                return Ok(CheckAllowlistResponse { allowed: true });
            }

            // First-time login: check admission (open mode admits all; restricted requires match)
            let allowed =
                crate::repos::allowed_emails::is_email_admitted(txn, &email, admission_mode)
                    .await
                    .map_err(AppError::from)?;

            if !allowed {
                return Err(AppError::email_not_allowed());
            }

            Ok(CheckAllowlistResponse { allowed: true })
        })
    })
    .await?;

    Ok(HttpResponse::Ok().json(result))
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/login").route(web::post().to(login)));
    cfg.service(web::resource("/refresh").route(web::post().to(refresh)));
    cfg.service(web::resource("/check-allowlist").route(web::post().to(check_allowlist)));
}
