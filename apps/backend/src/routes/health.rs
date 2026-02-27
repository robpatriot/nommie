use actix_web::{web, HttpResponse};

use crate::readiness::READINESS_RETRY_AFTER_SECS;
use crate::state::app_state::AppState;

// ── Public endpoints ───────────────────────────────────────────────

/// `GET /healthz` – liveness probe. Always 200 if the process is alive.
pub async fn healthz() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(serde_json::json!({ "status": "alive" }))
}

/// `GET /readyz` – readiness probe. 200 if ready, 503 if not.
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

// ── Internal endpoints ─────────────────────────────────────────────

/// `GET /internal/healthz` – rich liveness info.
pub async fn internal_healthz(app_state: web::Data<AppState>) -> HttpResponse {
    let body = app_state.readiness().to_internal_healthz_json();
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(body)
}

/// `GET /internal/readyz` – rich readiness info.
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

// ── Route configuration ────────────────────────────────────────────

/// Register public health routes on the given scope.
/// Called from main.rs for the root scope.
pub fn configure_public_health_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(web::resource("/healthz").route(web::get().to(healthz)))
        .service(web::resource("/readyz").route(web::get().to(readyz)));
}

/// Register internal health routes under `/internal`.
pub fn configure_internal_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(web::resource("/healthz").route(web::get().to(internal_healthz)))
        .service(web::resource("/readyz").route(web::get().to(internal_readyz)));
}
