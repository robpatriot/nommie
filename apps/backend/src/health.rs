use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use crate::AppError;

async fn root() -> impl actix_web::Responder {
    HttpResponse::Ok().body("Hello from Nommie Backend! 🃏")
}

async fn health() -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok().body("ok"))
}

async fn health_with_error(req: HttpRequest) -> Result<HttpResponse, AppError> {
    Err(AppError::invalid("INVALID_EXAMPLE", "Example failure".to_string())
        .with_trace_id(req.extensions().get::<String>().cloned()))
}

pub fn configure_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.route("/", web::get().to(root))
       .route("/health", web::get().to(health))
       .route("/health/error", web::get().to(health_with_error));
}
