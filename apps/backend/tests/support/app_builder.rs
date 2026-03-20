use actix_http::Request;
use actix_web::body::BoxBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, App, Error};
use backend::middleware::request_trace::RequestTrace;
use backend::middleware::structured_logger::StructuredLogger;
use backend::middleware::trace_span::TraceSpan;
use backend::state::app_state::AppState;
use backend::{routes, AppError};

use crate::support::test_middleware::TestSessionInjector;

/// Type alias for route configuration functions
type RouteConfigFn = Box<dyn Fn(&mut web::ServiceConfig) + Send + Sync>;

/// Configure all application routes for tests.
///
/// In production, `main.rs` wires these under scopes with additional
/// middleware (rate limiting, security headers, auth extractors). For
/// tests we register the same paths without those wrappers so that
/// endpoint behavior can be exercised directly.
///
/// Auth middleware is provided by TestSessionInjector injected per-scope
/// so that tests can supply a specific user identity.
fn configure_test_routes(cfg: &mut web::ServiceConfig) {
    // More specific /api/* scopes before generic /api (health) so e.g. /api/auth/login matches.
    cfg.service(web::scope("/api/auth").configure(routes::auth::configure_routes));
    cfg.service(web::scope("/api/games").configure(routes::games::configure_routes));
    cfg.service(
        web::scope("/api/user")
            .configure(routes::user::configure_routes)
            .configure(routes::user_options::configure_routes),
    );
    cfg.service(
        web::scope("/api/admin")
            .configure(routes::admin::configure_routes),
    );
    cfg.service(web::scope("/api/ws").configure(routes::realtime::configure_routes));

    cfg.service(
        web::scope("/api/internal").configure(routes::health::configure_api_internal_routes),
    );
    cfg.service(web::scope("/api").configure(routes::health::configure_api_health_routes));
}

/// Builder for creating test Actix service instances
pub struct TestAppBuilder {
    state: AppState,
    route_config: Option<RouteConfigFn>,
    session_injector: Option<TestSessionInjector>,
}

impl TestAppBuilder {
    /// Create a new TestAppBuilder with the given AppState
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            route_config: None,
            session_injector: None,
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

    /// Inject a session for all requests (replaces JWT-based auth in tests)
    pub fn with_session(mut self, injector: TestSessionInjector) -> Self {
        self.session_injector = Some(injector);
        self
    }

    /// Build the test service
    pub async fn build(
        self,
    ) -> Result<impl Service<Request, Response = ServiceResponse<BoxBody>, Error = Error>, AppError>
    {
        let state = self.state;
        let route_config = self.route_config;
        let session_injector = self.session_injector;

        // Wrap AppState with web::Data at the boundary
        let data = web::Data::new(state);

        let service = test::init_service(
            App::new()
                .wrap(StructuredLogger)
                .wrap(TraceSpan)
                .wrap(RequestTrace)
                .app_data(data)
                .configure(move |cfg| {
                    if let Some(injector) = session_injector.clone() {
                        // Wrap all routes with session injector when provided
                        cfg.service(
                            web::scope("")
                                .wrap(injector)
                                .configure(|inner_cfg| {
                                    if let Some(ref config_fn) = route_config {
                                        config_fn(inner_cfg);
                                    }
                                }),
                        );
                    } else if let Some(ref config_fn) = route_config {
                        config_fn(cfg);
                    }
                }),
        )
        .await;

        Ok(service)
    }
}

/// Create a new test app builder with the given AppState
pub fn create_test_app(state: AppState) -> TestAppBuilder {
    TestAppBuilder::new(state)
}
