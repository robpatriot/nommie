use actix_cors::Cors;
use actix_web::http::header;

pub fn cors_middleware() -> Cors {
    Cors::default()
        .allowed_origin("http://localhost:3000") // Frontend dev server
        .allowed_origin("http://127.0.0.1:3000") // Frontend dev server alternative
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
        ])
        .supports_credentials()
        .max_age(3600)
}
