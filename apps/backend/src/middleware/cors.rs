// apps/backend/src/middleware/cors.rs
use std::env;

use actix_cors::Cors;
use actix_web::http::header;

/// Build CORS middleware with a restrictive, explicit configuration:
/// - Origins must be configured via CORS_ALLOWED_ORIGINS
/// - Only allow methods actually used by the API
/// - Lightly validate origins, and ignore empty / \"null\" entries
pub fn cors_middleware() -> Cors {
    // Comma-separated origins, e.g.:
    // CORS_ALLOWED_ORIGINS=http://localhost:3000,http://127.0.0.1:3000,https://app.nommie.xyz
    let allowed_raw = env::var("CORS_ALLOWED_ORIGINS").unwrap_or_default();

    // Parse and lightly validate allowed origins (string-level only)
    let allowed_origins: Vec<String> = allowed_raw
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "null")
        .filter(|s| s.starts_with("http://") || s.starts_with("https://"))
        .map(|s| s.to_string())
        .collect();

    // Fallback to localhost-only if nothing valid was configured
    let effective_origins: Vec<String> = if allowed_origins.is_empty() {
        vec![
            "http://localhost:3000".to_string(),
            "http://127.0.0.1:3000".to_string(),
        ]
    } else {
        allowed_origins
    };

    let mut cors = Cors::default()
        // Methods actually used by the API
        .allowed_methods(vec!["GET", "POST", "PATCH", "DELETE", "OPTIONS"])
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

    // Add each validated origin explicitly
    for origin in effective_origins {
        cors = cors.allowed_origin(&origin);
    }

    cors
}
