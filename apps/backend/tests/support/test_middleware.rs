//! Test-only middleware for injecting a SharedTxn or SessionData into request extensions.
//!
//! TestTxnInjector is used by tests that need a full HTTP server (e.g., WebSocket tests)
//! where test code cannot directly construct/mutate the server-side HttpRequest.
//!
//! TestSessionInjector injects SessionData directly into request extensions, bypassing
//! Redis. Use it for handler tests that do not exercise SessionExtract end-to-end.

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, HttpMessage};
use backend::auth::session::SessionData;
use backend::db::txn::SharedTxn;
use futures_util::future::{ready, LocalBoxFuture, Ready};

// ── TestTxnInjector ──────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TestTxnInjector {
    shared: SharedTxn,
}

impl TestTxnInjector {
    pub fn new(shared: SharedTxn) -> Self {
        Self { shared }
    }
}

impl<S, B> Transform<S, ServiceRequest> for TestTxnInjector
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TestTxnInjectorMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TestTxnInjectorMiddleware {
            service,
            shared: self.shared.clone(),
        }))
    }
}

pub struct TestTxnInjectorMiddleware<S> {
    service: S,
    shared: SharedTxn,
}

impl<S, B> Service<ServiceRequest> for TestTxnInjectorMiddleware<S>
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
        req.extensions_mut().insert(self.shared.clone());

        // Call the downstream service
        let fut = self.service.call(req);
        Box::pin(fut)
    }
}

// ── TestSessionInjector ──────────────────────────────────────────────────────

/// Middleware that injects SessionData directly into request extensions.
///
/// Use this in tests that exercise route handlers or downstream logic but do not
/// need to test SessionExtract itself. No Redis connection required.
#[derive(Clone)]
pub struct TestSessionInjector {
    session_data: SessionData,
}

impl TestSessionInjector {
    pub fn new(user_id: i64, sub: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            session_data: SessionData {
                user_id,
                sub: sub.into(),
                email: email.into(),
            },
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for TestSessionInjector
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TestSessionInjectorMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TestSessionInjectorMiddleware {
            service,
            session_data: self.session_data.clone(),
        }))
    }
}

pub struct TestSessionInjectorMiddleware<S> {
    service: S,
    session_data: SessionData,
}

impl<S, B> Service<ServiceRequest> for TestSessionInjectorMiddleware<S>
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
        req.extensions_mut().insert(self.session_data.clone());

        let fut = self.service.call(req);
        Box::pin(fut)
    }
}
