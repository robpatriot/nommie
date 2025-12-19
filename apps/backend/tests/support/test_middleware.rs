//! Test-only middleware for injecting a SharedTxn into request extensions.
//!
//! This is used by tests that need a full HTTP server (e.g., WebSocket tests) where test
//! code cannot directly construct/mutate the server-side HttpRequest. The middleware inserts
//! a pre-created SharedTxn into request extensions so handlers can reuse it via `with_txn()`.

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, HttpMessage};
use backend::db::txn::SharedTxn;
use futures_util::future::{ready, LocalBoxFuture, Ready};

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
