use std::future::{ready, Ready};

use actix_web::body::EitherBody;
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{HeaderValue, CACHE_CONTROL};
use actix_web::{web, HttpMessage};
use futures_util::future::LocalBoxFuture;

use crate::error::{AppError, Sentinel};
use crate::readiness::READINESS_RETRY_AFTER_SECS;
use crate::state::app_state::AppState;

/// Middleware that gates `/api/*` requests behind readiness.
///
/// When the service is NOT ready, returns `503 Service Unavailable` immediately
/// with a JSON body.  Health and internal endpoints are handled on separate
/// scopes and are **not** wrapped with this middleware.
pub struct ReadinessGate;

impl<S, B> Transform<S, ServiceRequest> for ReadinessGate
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = ReadinessGateMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ReadinessGateMiddleware { service }))
    }
}

pub struct ReadinessGateMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for ReadinessGateMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Try to get the ReadinessManager from app data
        let is_ready = req
            .app_data::<web::Data<AppState>>()
            .map(|state| state.readiness().is_ready())
            .unwrap_or(false);

        if !is_ready {
            let trace_id = req
                .extensions()
                .get::<String>()
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            let err = AppError::db_unavailable(
                "Service temporarily unavailable",
                Sentinel("readiness gate blocked"),
                Some(READINESS_RETRY_AFTER_SECS),
            );
            let mut response = err.to_http_response_with_trace_id(trace_id);
            response
                .headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
            return Box::pin(async { Ok(req.into_response(response).map_into_right_body()) });
        }

        let fut = self.service.call(req);
        Box::pin(async move { fut.await.map(ServiceResponse::map_into_left_body) })
    }
}
