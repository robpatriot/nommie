use actix_web::web;

pub mod auth;
pub mod games;
pub mod health;
pub mod user_options;

pub fn configure(cfg: &mut web::ServiceConfig) {
    // Health check routes only - no rate limiting or auth required
    cfg.configure(health::configure_routes);
    // Note: auth, games, and user routes are configured separately in main.rs
    // with their respective rate limiting and auth middleware
}
