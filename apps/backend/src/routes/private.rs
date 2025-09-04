use actix_web::{web, HttpResponse, Result};
use serde::Serialize;

use crate::error::AppError;
use crate::extractors::current_user::CurrentUser;
use crate::extractors::current_user_db::CurrentUserRecord;

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub sub: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct MeDbResponse {
    pub id: i64,
    pub sub: String,
    pub email: Option<String>,
}

/// Protected endpoint that returns the caller's identity (claims-only)
async fn me(auth: CurrentUser) -> Result<HttpResponse, AppError> {
    let response = MeResponse {
        sub: auth.sub,
        email: auth.email,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Protected endpoint that returns the caller's identity from database
async fn me_db(
    _auth: CurrentUser,
    user_record: CurrentUserRecord,
) -> Result<HttpResponse, AppError> {
    let response = MeDbResponse {
        id: user_record.id,
        sub: user_record.sub,
        email: user_record.email,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/api/private/me").route(web::get().to(me)));
    cfg.service(web::resource("/api/private/me_db").route(web::get().to(me_db)));
}
