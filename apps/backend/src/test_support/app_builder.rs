//! Test service builder (two-stage test harness, stage 2).
//! Given an AppState, build an initialized Actix **test service**.

use actix_web::body::BoxBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::Error as ActixError;
use actix_web::{
    web::{self, ServiceConfig},
    App,
};

use crate::error::AppError;
use crate::state::app_state::AppState;

/// Function pointer for custom route configuration.
type RoutesFn = Box<dyn FnOnce(&mut ServiceConfig) + Send>;

pub fn create_test_app_builder(state: AppState) -> TestAppBuilder {
    TestAppBuilder {
        state,
        router: Router::Unset,
    }
}

enum Router {
    Unset,
    Prod,
    Custom(RoutesFn),
}

pub struct TestAppBuilder {
    state: AppState,
    router: Router,
}

impl TestAppBuilder {
    /// Use the application's production routes.
    pub fn with_prod_routes(mut self) -> Self {
        self.router = Router::Prod;
        self
    }

    /// Use custom routes for a test.
    pub fn with_routes<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut ServiceConfig) + Send + 'static,
    {
        self.router = Router::Custom(Box::new(f));
        self
    }

    /// Build and initialize the Actix test service.
    ///
    /// Return type is `impl Service<...>` so callers don't have to name the opaque service type.
    pub async fn build(
        self,
    ) -> Result<
        impl Service<actix_http::Request, Response = ServiceResponse<BoxBody>, Error = ActixError>,
        AppError,
    > {
        let mut app = App::new()
            // Register a single Data<AppState>.
            .app_data(web::Data::new(self.state.clone()));

        app = match self.router {
            Router::Unset | Router::Prod => {
                // ⬇️ Adjust this to your real prod-router function.
                // e.g., crate::http::routes::configure, crate::routes::prod, etc.
                app.configure(crate::routes::configure)
            }
            Router::Custom(f) => app.configure(f),
        };

        let srv = actix_web::test::init_service(app).await;
        Ok(srv)
    }
}
