use actix_web::{web, HttpResponse};
use crate::AppError;

async fn root() -> impl actix_web::Responder {
    HttpResponse::Ok().body("Hello from Nommie Backend! 🃏")
}

async fn health() -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok().body("ok"))
}

pub fn configure_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.route("/", web::get().to(root))
       .route("/health", web::get().to(health));
}
