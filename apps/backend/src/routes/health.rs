use actix_web::{web, HttpResponse};

use crate::readiness::READINESS_RETRY_AFTER_SECS;
use crate::state::app_state::AppState;

/// `GET /api/livez` – liveness probe. Always 200 if the process is alive.
pub async fn livez() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(serde_json::json!({ "status": "alive" }))
}

/// `GET /api/readyz` – readiness probe. 200 if ready, 503 if not.
pub async fn readyz(app_state: web::Data<AppState>) -> HttpResponse {
    let manager = app_state.readiness();
    let body = manager.to_public_json();
    let is_ready = manager.is_ready();

    if is_ready {
        HttpResponse::Ok()
            .insert_header(("Cache-Control", "no-store"))
            .json(body)
    } else {
        HttpResponse::ServiceUnavailable()
            .insert_header(("Cache-Control", "no-store"))
            .insert_header(("Retry-After", READINESS_RETRY_AFTER_SECS.to_string()))
            .json(body)
    }
}

/// `GET /api/internal/readyz` – rich readiness info (humans/devops).
pub async fn internal_readyz(app_state: web::Data<AppState>) -> HttpResponse {
    let manager = app_state.readiness();
    let body = manager.to_internal_json();
    let is_ready = manager.is_ready();

    if is_ready {
        HttpResponse::Ok()
            .insert_header(("Cache-Control", "no-store"))
            .json(body)
    } else {
        HttpResponse::ServiceUnavailable()
            .insert_header(("Cache-Control", "no-store"))
            .insert_header(("Retry-After", READINESS_RETRY_AFTER_SECS.to_string()))
            .json(body)
    }
}

/// Register API health routes: /api/livez, /api/readyz.
pub fn configure_api_health_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(web::resource("/livez").route(web::get().to(livez)))
        .service(web::resource("/readyz").route(web::get().to(readyz)));
}

/// Register API internal routes: /api/internal/readyz.
pub fn configure_api_internal_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(web::resource("/readyz").route(web::get().to(internal_readyz)));
}
