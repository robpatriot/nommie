use actix_http::Request;
use actix_web::body::BoxBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, App, Error};
use backend::AppError;
use backend::middleware::request_trace::RequestTrace;
use backend::middleware::structured_logger::StructuredLogger;
use backend::middleware::trace_span::TraceSpan;
use backend::routes;
use backend::state::app_state::AppState;

/// Type alias for route configuration functions
type RouteConfigFn = Box<dyn Fn(&mut web::ServiceConfig) + Send + Sync>;

/// Configure all application routes for tests.
///
/// In production, `main.rs` wires these under scopes with additional
/// middleware (rate limiting, security headers, auth extractors). For
/// tests we register the same paths without those wrappers so that
/// endpoint behavior can be exercised directly.
fn configure_test_routes(cfg: &mut web::ServiceConfig) {
    // Health check routes: /health
    cfg.service(web::scope("/health").configure(routes::health::configure_routes));

    // Auth routes: /api/auth/**
    cfg.service(web::scope("/api/auth").configure(routes::auth::configure_routes));

    // Games routes: /api/games/**
    cfg.service(web::scope("/api/games").configure(routes::games::configure_routes));

    // User routes: /api/user/**
    cfg.service(web::scope("/api/user").configure(routes::user_options::configure_routes));

    // Realtime routes: /api/ws/**
    cfg.service(web::scope("/api/ws").configure(routes::realtime::configure_routes));
}

/// Builder for creating test Actix service instances
pub struct TestAppBuilder {
    state: AppState,
    route_config: Option<RouteConfigFn>,
}

impl TestAppBuilder {
    /// Create a new TestAppBuilder with the given AppState
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            route_config: None,
        }
    }

    /// Configure the app to use production routes
    pub fn with_prod_routes(mut self) -> Self {
        self.route_config = Some(Box::new(configure_test_routes) as RouteConfigFn);
        self
    }

    /// Configure the app with custom routes
    pub fn with_routes<F>(mut self, config_fn: F) -> Self
    where
        F: Fn(&mut web::ServiceConfig) + Send + Sync + 'static,
    {
        self.route_config = Some(Box::new(config_fn) as RouteConfigFn);
        self
    }

    /// Build the test service
    pub async fn build(
        self,
    ) -> Result<impl Service<Request, Response = ServiceResponse<BoxBody>, Error = Error>, AppError>
    {
        let state = self.state;
        let route_config = self.route_config;

        // Wrap AppState with web::Data at the boundary
        let data = web::Data::new(state);

        let service = test::init_service(
            App::new()
                .wrap(StructuredLogger)
                .wrap(TraceSpan)
                .wrap(RequestTrace)
                .app_data(data)
                .configure(move |cfg| {
                    if let Some(config_fn) = &route_config {
                        config_fn(cfg);
                    }
                }),
        )
        .await;

        Ok(service)
    }
}

/// Create a new test app builder with the given AppState
///
/// # Example
/// ```rust
/// use backend::infra::state::build_state;
/// use support::app_builder::create_test_app;
/// use backend::config::db::{RuntimeEnv, DbKind};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use crate::support::test_state::build_test_state;
///
/// let state = build_test_state().await?;
/// let app = create_test_app(state).with_prod_routes().build().await?;
/// # Ok(())
/// # }
/// ```
pub fn create_test_app(state: AppState) -> TestAppBuilder {
    TestAppBuilder::new(state)
}
