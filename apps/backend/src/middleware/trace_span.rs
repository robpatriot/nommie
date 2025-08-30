//! Per-request tracing span middleware.
//!
//! Creates a span named "request" carrying `trace_id`, `method`, and `path`,
//! and instruments the downstream future so all logs inside handlers
//! automatically inherit these fields.
//!
//! Ordering: this middleware expects `RequestTrace` to have already
//! inserted a `String` trace_id into `req.extensions()`. Therefore,
//! wire it **after** `RequestTrace`, e.g.:
//!
//! App::new()
//!     .wrap(StructuredLogger)
//!     .wrap(RequestTrace)   // generates + stores trace_id, sets header
//!     .wrap(TraceSpan)      // reads trace_id and creates the span
//!     // routes...

use std::future::{ready, Ready};
use std::task::{Context, Poll};

use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error;
use actix_web::HttpMessage; // 👈 bring trait for `extensions()`
use futures_util::future::LocalBoxFuture;
use tracing::{info_span, Instrument, Span};

/// Middleware type (unit struct is fine; no config needed).
#[derive(Clone, Default)]
pub struct TraceSpan;

impl<S, B> Transform<S, ServiceRequest> for TraceSpan
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TraceSpanMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TraceSpanMiddleware { service }))
    }
}

pub struct TraceSpanMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for TraceSpanMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // ---- Extract fields for the span
        // `RequestTrace` should have inserted a String trace_id into extensions.
        let trace_id = req
            .extensions()
            .get::<String>()
            .cloned()
            .unwrap_or_else(|| "missing-trace-id".to_string());

        let method = req.method().clone();
        let path = req.path().to_string();

        // ---- Create the span
        let span: Span = info_span!(
            "request",
            trace_id = %trace_id,
            method = %method,
            path = %path
        );

        // ---- Call downstream, instrumenting the future so the span stays active
        let fut = self.service.call(req).instrument(span);

        Box::pin(fut)
    }
}
