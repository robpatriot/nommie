use actix_web::web;

pub mod auth;
pub mod games;
pub mod health;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(auth::configure_routes)
        .configure(health::configure_routes);
    // Note: games routes are configured separately in main.rs with JwtExtract middleware
}
