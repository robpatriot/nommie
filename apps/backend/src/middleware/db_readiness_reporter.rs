//! Middleware that reports database availability issues to the ReadinessManager.
//!
//! This middleware inspects errors returned by handlers. When a handler fails
//! with an `AppError::DbUnavailable` (or a timeout classified as
//! `ErrorCode::DbTimeout`), it reports the Postgres dependency as `Down` to
//! the readiness manager so that `/api/readyz` can transition away from
//! `Healthy` after sustained failures.

use std::time::Duration;

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, Error};
use futures_util::future::{ready, LocalBoxFuture, Ready};

use crate::error::DbOutageMarker;
use crate::readiness::types::{DependencyCheck, DependencyName};
use crate::state::app_state::AppState;

/// Middleware marker type.
pub struct DbReadinessReporter;

impl<S, B> Transform<S, ServiceRequest> for DbReadinessReporter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = DbReadinessReporterMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(DbReadinessReporterMiddleware { service }))
    }
}

/// Inner middleware type.
pub struct DbReadinessReporterMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for DbReadinessReporterMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let app_state = req.app_data::<web::Data<AppState>>().cloned();

        let fut = self.service.call(req);

        Box::pin(async move {
            let result = fut.await;

            match &result {
                Ok(res) => {
                    // Check for an internal marker indicating this response was generated
                    // due to a database outage (DbUnavailable / DbTimeout).
                    let has_db_outage_marker = res
                        .response()
                        .extensions()
                        .get::<DbOutageMarker>()
                        .is_some();

                    if has_db_outage_marker {
                        if let Some(state) = app_state {
                            let latency = Duration::ZERO;
                            let error = "database outage reported by AppError".to_string();

                            let transitioned = state.readiness().update_dependency(
                                DependencyName::Postgres,
                                DependencyCheck::Down { error, latency },
                            );

                            if transitioned {
                                state.readiness().wake_monitor();
                            }
                        }
                    }
                }
                Err(_err) => {
                    // Actix error; no readiness reporting here (handler didn't return AppError).
                }
            }

            result
        })
    }
}
