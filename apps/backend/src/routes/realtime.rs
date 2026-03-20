use actix_web::{web, HttpResponse};
use serde::Serialize;

use crate::auth::session::{generate_session_token, store_ws_token, SessionData};
use crate::error::AppError;
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
            crate::errors::ErrorCode::InternalError,
            "Current user missing email".to_string(),
            std::io::Error::other("missing email"),
        )
    })?;

    let mut conn = app_state.session_redis().ok_or_else(|| {
        AppError::redis_unavailable(
            "session Redis not available".to_string(),
            crate::error::Sentinel("session Redis not available"),
            None,
        )
    })?;

    let token = generate_session_token();
    let data = SessionData {
        user_id: current_user.id,
        sub: current_user.sub.clone(),
        email,
    };
    store_ws_token(&mut conn, &token, &data).await?;

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
