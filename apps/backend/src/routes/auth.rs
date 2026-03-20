use actix_web::{web, HttpRequest, HttpResponse, Result};
use serde::{Deserialize, Serialize};

use crate::auth::session::{generate_session_token, store_session, SessionData};
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

/// Handle Google OAuth login with verified ID token.
/// Verifies the token server-side, extracts trusted claims, and returns a session token.
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

    let mut conn = app_state.session_redis().ok_or_else(|| {
        AppError::redis_unavailable(
            "session Redis not available".to_string(),
            crate::error::Sentinel("session Redis not available"),
            None,
        )
    })?;

    let token = generate_session_token();
    let data = SessionData {
        user_id: user.id,
        sub: claims.sub.clone(),
        email: claims.email.clone(),
    };
    store_session(&mut conn, &token, &data).await?;

    let response = LoginResponse { token };
    Ok(HttpResponse::Ok().json(response))
}

/// Logout: delete session from Redis and return 200.
async fn logout(
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    if let Some(mut conn) = app_state.session_redis() {
        if let Some(cookie) = req.cookie("backend_session") {
            crate::auth::session::delete_session(&mut conn, cookie.value()).await;
        }
    }
    Ok(HttpResponse::Ok().finish())
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/login").route(web::post().to(login)));
    cfg.service(web::resource("/logout").route(web::post().to(logout)));
}
