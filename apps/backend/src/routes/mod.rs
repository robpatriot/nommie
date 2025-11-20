use actix_web::web;

use crate::middleware::jwt_extract::JwtExtract;

pub mod auth;
pub mod games;
pub mod health;
pub mod user_options;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(auth::configure_routes)
        .configure(health::configure_routes)
        .service(
            web::scope("/api/user")
                .wrap(JwtExtract)
                .configure(user_options::configure_routes),
        );
    // Note: games routes are configured separately in main.rs with JwtExtract middleware
}
