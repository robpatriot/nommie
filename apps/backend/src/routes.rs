use actix_web::web;

pub mod auth;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(crate::health::configure_routes)
        .configure(auth::configure_routes);
}
