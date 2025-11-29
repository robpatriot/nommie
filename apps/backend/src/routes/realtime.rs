use std::time::SystemTime;

use actix_web::{web, HttpResponse};
use serde::Serialize;

use crate::auth::jwt::mint_access_token_with_ttl;
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::extractors::current_user::CurrentUser;
use crate::state::app_state::AppState;

const WS_TOKEN_TTL_SECONDS: i64 = 90;

#[derive(Serialize)]
struct WsTokenResponse {
    token: String,
    expires_in: i64,
}

async fn issue_ws_token(
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let email = current_user.email.clone().ok_or_else(|| {
        AppError::internal(
            ErrorCode::InternalError,
            "Current user missing email".to_string(),
            std::io::Error::other("missing email"),
        )
    })?;

    let token = mint_access_token_with_ttl(
        &current_user.sub,
        &email,
        SystemTime::now(),
        WS_TOKEN_TTL_SECONDS,
        &app_state.security,
    )?;

    Ok(HttpResponse::Ok().json(WsTokenResponse {
        token,
        expires_in: WS_TOKEN_TTL_SECONDS,
    }))
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/token")
            .route(web::post().to(issue_ws_token))
            .route(web::get().to(issue_ws_token)),
    );
}
