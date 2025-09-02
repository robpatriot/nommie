pub mod app_builder;
pub mod factories;
pub mod migrations;
pub mod schema_guard;
pub mod state_builder;

use crate::{middleware::RequestTrace, state::AppState};
use actix_http::Request;
use actix_web::body::BoxBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, App, Error};

/// Load test env from .env.test if present (no panic if missing).
pub fn load_test_env() {
    dotenvy::from_filename(".env.test").ok();
}

/// Panic unless the DB URL ends with `_test`.
pub fn assert_test_db_url(url: &str) {
    if !url.ends_with("_test") {
        panic!("Tests must run against a test database ending with `_test`. Current DATABASE_URL: {url}");
    }
}

/// Returns the validated test DB URL from `DATABASE_URL`.
/// - Loads `.env.test`
/// - Reads `DATABASE_URL`
/// - Asserts `_test` suffix
pub fn get_test_db_url() -> String {
    use std::env;
    load_test_env();
    let url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!("DATABASE_URL must be set for tests (pnpm test should export it)")
    });
    assert_test_db_url(&url);
    url
}

pub async fn create_test_app(
    state: web::Data<AppState>,
) -> impl Service<Request, Response = ServiceResponse<BoxBody>, Error = Error> {
    test::init_service(
        App::new()
            .wrap(RequestTrace)
            .app_data(state) // Data<T> is Arc under the hood
            .configure(crate::routes::configure),
    )
    .await
}

// Re-export the new builders
pub use app_builder::create_test_app_builder;
pub use state_builder::create_test_state;
