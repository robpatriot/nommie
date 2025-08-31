use actix_web::web;

pub mod auth;
pub mod private;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(crate::health::configure_routes)
        .configure(auth::configure_routes)
        .configure(private::configure_routes);
}
