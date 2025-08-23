use actix_web::{get, App, HttpResponse, Responder};

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("ok")
}

/// Builds the application configuration closure
/// Returns a closure that configures the App with the health route
/// This avoids the generic type issues with actix_web::App
pub fn build_app() -> impl FnOnce() -> actix_web::App<impl actix_web::dev::ServiceFactory> {
    || App::new().service(health)
}
