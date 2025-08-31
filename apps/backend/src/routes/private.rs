use actix_web::{web, HttpResponse, Result};
use serde::Serialize;

use crate::{error::AppError, extractors::BackendAuth};

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub sub: uuid::Uuid,
    pub email: String,
}

/// Protected endpoint that returns the caller's identity
async fn me(auth: BackendAuth) -> Result<HttpResponse, AppError> {
    let response = MeResponse {
        sub: auth.sub,
        email: auth.email,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/api/private/me").route(web::get().to(me)));
}
