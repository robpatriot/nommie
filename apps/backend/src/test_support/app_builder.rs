use crate::{error::AppError, middleware::RequestTrace, routes, state::AppState};
use actix_http::Request;
use actix_web::body::BoxBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, App, Error};

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

        let service = test::init_service(
            App::new()
                .wrap(RequestTrace)
                .app_data(web::Data::new(state))
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
/// use backend::test_support::{create_test_app_builder, create_test_state};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let state = create_test_state().with_db().build().await?;
/// let app = create_test_app_builder(state.clone()).with_prod_routes().build().await?;
/// # Ok(())
/// # }
/// ```
pub fn create_test_app_builder(state: AppState) -> TestAppBuilder {
    TestAppBuilder::new(state)
}
