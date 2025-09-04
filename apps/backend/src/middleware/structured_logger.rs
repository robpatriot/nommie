use std::time::Instant;

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use tracing::info;

pub struct StructuredLogger;

impl<S, B> Transform<S, ServiceRequest> for StructuredLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
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
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = Instant::now();
        let method = req.method().to_string();
        let path = req.path().to_string();

        let fut = self.service.call(req);

        Box::pin(async move {
            let result = fut.await;

            let latency = start_time.elapsed();
            let latency_ms = latency.as_millis() as u64;

            match &result {
                Ok(res) => {
                    let status_code = res.status().as_u16();

                    // Extract trace_id from response headers (set by RequestTrace middleware)
                    let trace_id = res
                        .headers()
                        .get("x-request-id")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown");

                    info!(
                        method = %method,
                        path = %path,
                        status_code = %status_code,
                        latency_ms = %latency_ms,
                        trace_id = %trace_id,
                        message = "request_completed"
                    );
                }
                Err(_) => {
                    // Log error requests with status 500 (typical for unhandled errors)
                    // For errors, we can't access response headers, so use "unknown"
                    info!(
                        method = %method,
                        path = %path,
                        status_code = 500,
                        latency_ms = %latency_ms,
                        trace_id = "unknown",
                        message = "request_completed"
                    );
                }
            }

            result
        })
    }
}
