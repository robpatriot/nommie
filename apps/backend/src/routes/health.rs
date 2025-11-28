use actix_web::{web, HttpResponse};
use migration::get_latest_migration_version;
use sea_orm::ConnectionTrait;
use serde::Serialize;
use time::OffsetDateTime;

use crate::db::require_db;
use crate::error::AppError;
use crate::state::app_state::AppState;

pub async fn root() -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok().body("Hello from Nommie Backend! 🃏"))
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    app_version: String,
    db: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    db_error: Option<String>,
    migrations: String,
    time: String,
}

async fn health(app_state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    // Get app version from Cargo.toml
    let app_version = env!("CARGO_PKG_VERSION").to_string();

    // Get current time in ISO 8601 format
    let now = OffsetDateTime::now_utc();
    let time = now
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());

    // Check database connectivity and get migration status
    let (db_status, db_error, migrations) = match require_db(&app_state) {
        Ok(db) => {
            // Try a lightweight query to verify connection
            match db
                .query_one(sea_orm::Statement::from_string(
                    db.get_database_backend(),
                    "SELECT 1 as health_check".to_string(),
                ))
                .await
            {
                Ok(_) => {
                    // Query migration status using the new migration function
                    let migration_version = match get_latest_migration_version(db).await {
                        Ok(Some(version)) => version,
                        Ok(None) => "no_migrations".to_string(),
                        Err(_) => "unknown".to_string(),
                    };
                    ("ok".to_string(), None, migration_version)
                }
                Err(e) => (
                    "error".to_string(),
                    Some(format!("DB query failed: {e}")),
                    "unknown".to_string(),
                ),
            }
        }
        Err(e) => (
            "error".to_string(),
            Some(format!("DB unavailable: {e}")),
            "unknown".to_string(),
        ),
    };

    let response = HealthResponse {
        status: "ok".to_string(),
        app_version,
        db: db_status,
        db_error,
        migrations,
        time,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub fn configure_routes(cfg: &mut actix_web::web::ServiceConfig) {
    // Only configure health route - root route is configured separately in main.rs
    cfg.route("/health", web::get().to(health));
}
