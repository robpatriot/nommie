use actix_web::{web, HttpResponse, Responder};

async fn health() -> impl Responder {
    HttpResponse::Ok().body("ok")
}

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.route("/health", web::get().to(health));
}
