use actix_web::web;

pub mod auth;
pub mod health;
pub mod private;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(auth::configure_routes)
        .configure(health::configure_routes)
        .configure(private::configure_routes);
}
