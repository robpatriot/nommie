use std::time::Instant;
use std::future::{ready, Ready};

use actix_web::{HttpMessage, Error as ActixError};
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use futures_util::future::LocalBoxFuture;
use tracing::{info, warn, error};

pub struct StructuredLogger;

impl<S, B> Transform<S, ServiceRequest> for StructuredLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type InitError = ();
    type Transform = StructuredLoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(StructuredLoggerMiddleware { service }))
    }
}

pub struct StructuredLoggerMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for StructuredLoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = Instant::now();
        let method = req.method().to_string();
        let path = req.path().to_string();

        let trace_id = req
            .extensions()
            .get::<String>()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let fut = self.service.call(req);

        Box::pin(async move {
            let result = fut.await;

            let status = match &result {
                Ok(res) => res.status(),
                Err(err) => err.as_response_error().status_code(),
            };

            let duration_us = start.elapsed().as_micros() as u64;
            let status_code = status.as_u16();

            if status.is_server_error() {
                error!(http.method=%method, url.path=%path, http.status_code=%status_code, duration_us=%duration_us, trace_id=%trace_id, message="request_completed");
            } else if status.is_client_error() {
                warn!(http.method=%method, url.path=%path, http.status_code=%status_code, duration_us=%duration_us, trace_id=%trace_id, message="request_completed");
            } else {
                info!(http.method=%method, url.path=%path, http.status_code=%status_code, duration_us=%duration_us, trace_id=%trace_id, message="request_completed");
            }

            result
        })
    }
}
