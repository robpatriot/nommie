use crate::AppError;
use actix_web::{web, HttpResponse};

async fn root() -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok().body("Hello from Nommie Backend! 🃏"))
}

async fn health() -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok().body("ok"))
}

pub fn configure_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.route("/", web::get().to(root))
        .route("/health", web::get().to(health));
}
