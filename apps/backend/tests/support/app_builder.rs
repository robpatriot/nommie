use actix_http::Request;
use actix_web::body::BoxBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, App, Error};
use backend::error::AppError;
use backend::middleware::request_trace::RequestTrace;
use backend::routes;
use backend::state::app_state::AppState;

/// Type alias for route configuration functions
type RouteConfigFn = Box<dyn Fn(&mut web::ServiceConfig) + Send + Sync>;

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
        self.route_config = Some(Box::new(routes::configure) as RouteConfigFn);
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

        let service = test::init_service(App::new().wrap(RequestTrace).app_data(data).configure(
            move |cfg| {
                if let Some(config_fn) = &route_config {
                    config_fn(cfg);
                }
            },
        ))
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
/// use backend::config::db::DbProfile;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let state = build_state().with_db(DbProfile::Test).build().await?;
/// let app = create_test_app(state).with_prod_routes().build().await?;
/// # Ok(())
/// # }
/// ```
pub fn create_test_app(state: AppState) -> TestAppBuilder {
    TestAppBuilder::new(state)
}
