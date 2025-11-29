use actix_web::web;

pub mod auth;
pub mod games;
pub mod health;
pub mod realtime;
pub mod user_options;

/// Configure application routes for tests and non-HttpServer contexts.
///
/// In production, `main.rs` wires these under scopes with additional
/// middleware (rate limiting, security headers, auth extractors). For
/// tests we register the same paths without those wrappers so that
/// endpoint behavior can be exercised directly.
pub fn configure(cfg: &mut web::ServiceConfig) {
    // Health check routes: /health
    cfg.service(web::scope("/health").configure(health::configure_routes));

    // Auth routes: /api/auth/**
    cfg.service(web::scope("/api/auth").configure(auth::configure_routes));

    // Games routes: /api/games/**
    cfg.service(web::scope("/api/games").configure(games::configure_routes));

    // User routes: /api/user/**
    cfg.service(web::scope("/api/user").configure(user_options::configure_routes));

    // Realtime routes: /api/ws/**
    cfg.service(web::scope("/api/ws").configure(realtime::configure_routes));
}
