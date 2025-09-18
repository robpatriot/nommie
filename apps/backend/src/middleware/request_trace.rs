use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header;
use actix_web::HttpMessage;
use futures_util::future::{ready, LocalBoxFuture, Ready};
use uuid::Uuid;

use crate::web::trace_ctx;

pub struct RequestTrace;

impl<S, B> Transform<S, ServiceRequest> for RequestTrace
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = RequestTraceMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestTraceMiddleware { service }))
    }
}

pub struct RequestTraceMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestTraceMiddleware<S>
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
        let trace_id = Uuid::new_v4().to_string();

        // Insert trace_id into request extensions for other middleware (e.g., TraceSpan)
        req.extensions_mut().insert(trace_id.clone());

        let fut = self.service.call(req);

        Box::pin(async move {
            // Wrap the downstream future in the task-local trace context
            let res = trace_ctx::with_trace_id(trace_id.clone(), fut).await?;

            let mut res = res;

            // Add X-Trace-Id header to response
            res.headers_mut().insert(
                header::HeaderName::from_static("x-trace-id"),
                header::HeaderValue::from_str(&trace_id)
                    .unwrap_or_else(|_| header::HeaderValue::from_static("invalid-uuid")),
            );

            Ok(res)
        })
    }
}
