//! Per-request tracing span middleware.
//!
//! Creates a span named "request" carrying `trace_id`, `method`, `path`,
//! and optionally `user_id` and `game_id`, and instruments the downstream
//! future so all logs inside handlers automatically inherit these fields.
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
use actix_web::http::header;
use actix_web::Error;
use actix_web::HttpMessage; // 👈 bring trait for `extensions()`
use futures_util::future::LocalBoxFuture;
use tracing::{info_span, Instrument, Span};

use crate::auth::jwt::extract_sub_for_logging;

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

        // ---- Extract user_id from Authorization header (JWT)
        let user_id = extract_user_id_from_jwt(&req);

        // ---- Extract game_id from path parameters
        let game_id = req
            .match_info()
            .get("game_id")
            .and_then(|id_str| id_str.parse::<i64>().ok());

        // ---- Create the span with optional user_id and game_id
        let span: Span = info_span!(
            "request",
            trace_id = %trace_id,
            method = %method,
            path = %path,
            user_id = user_id.as_deref(),
            game_id = game_id
        );

        // ---- Call downstream, instrumenting the future so the span stays active
        let fut = self.service.call(req).instrument(span);

        Box::pin(fut)
    }
}

/// Extract user_id (sub claim) from the Bearer token in the Authorization header.
///
/// This is a lightweight helper for observability only - it does NOT validate
/// expiration or perform authorization checks. For authentication, use the
/// CurrentUser or JwtClaims extractors instead.
fn extract_user_id_from_jwt(req: &ServiceRequest) -> Option<String> {
    // Get Authorization header
    let auth_header = req.headers().get(header::AUTHORIZATION)?;
    let auth_str = auth_header.to_str().ok()?;

    // Extract token from "Bearer <token>"
    let token = auth_str.strip_prefix("Bearer ")?.trim();
    if token.is_empty() {
        return None;
    }

    // Get JWT secret from AppState
    let state = req.app_data::<actix_web::web::Data<crate::state::app_state::AppState>>()?;
    let secret = &state.security.jwt_secret;

    // Use shared utility to extract sub claim for logging
    extract_sub_for_logging(token, secret)
}
