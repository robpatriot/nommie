// apps/backend/src/middleware/cors.rs
use std::env;

use actix_cors::Cors;
use actix_web::http::header;

pub fn cors_middleware() -> Cors {
    // Comma-separated origins, e.g.:
    // CORS_ALLOWED_ORIGINS=http://localhost:3000,http://127.0.0.1:3000,https://app.nommie.xyz
    let allowed = env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://127.0.0.1:3000".to_string());

    let mut cors = Cors::default()
        // Allow common methods used by your API
        .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
        // Headers the browser may send
        .allowed_headers(vec![
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::IF_MATCH,
            header::IF_NONE_MATCH,
        ])
        // Headers the browser is allowed to read from responses
        .expose_headers(vec![
            header::HeaderName::from_static("x-trace-id"),
            header::ETAG,
        ])
        .max_age(3600);

    // Add each origin explicitly
    for origin in allowed
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        cors = cors.allowed_origin(origin);
    }

    cors
}
