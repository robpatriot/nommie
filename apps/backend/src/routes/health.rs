use actix_web::{web, HttpResponse};
use sea_orm::{ConnectionTrait, Statement};
use serde::Serialize;
use time::OffsetDateTime;

use crate::db::require_db;
use crate::error::AppError;
use crate::state::app_state::AppState;

async fn root() -> Result<HttpResponse, AppError> {
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
                .query_one(Statement::from_string(
                    sea_orm::DatabaseBackend::Postgres,
                    "SELECT 1 as health_check".to_string(),
                ))
                .await
            {
                Ok(_) => {
                    // Query migration status
                    let migration_version = match db
                        .query_one(Statement::from_string(
                            sea_orm::DatabaseBackend::Postgres,
                            "SELECT version FROM seaql_migrations ORDER BY version DESC LIMIT 1"
                                .to_string(),
                        ))
                        .await
                    {
                        Ok(Some(row)) => row
                            .try_get::<String>("", "version")
                            .unwrap_or_else(|_| "unknown".to_string()),
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
    cfg.route("/", web::get().to(root))
        .route("/health", web::get().to(health));
}
