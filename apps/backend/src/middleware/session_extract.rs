//! Session extraction middleware
//!
//! Validates the `backend_session` cookie against Redis session store,
//! or `?token=` query param against the ws_token namespace.

use std::collections::HashMap;
use std::rc::Rc;

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, Error, HttpMessage};
use futures_util::future::{ready, LocalBoxFuture, Ready};

use crate::auth::session::{get_and_slide_session, get_ws_token};
use crate::error::AppError;
use crate::state::app_state::AppState;

pub struct SessionExtract;

impl<S, B> Transform<S, ServiceRequest> for SessionExtract
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SessionExtractMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SessionExtractMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct SessionExtractMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for SessionExtractMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        let app_state = req.app_data::<web::Data<AppState>>().cloned();
        let cookie_token = req
            .cookie("backend_session")
            .map(|c| c.value().to_string());
        let query_token = extract_token_from_query(req.uri().query());

        let (token, is_ws_token) = match (cookie_token, query_token) {
            (Some(t), _) => (t, false),
            (None, Some(t)) => (t, true),
            (None, None) => {
                return Box::pin(async move {
                    Err(actix_web::Error::from(AppError::unauthorized()))
                });
            }
        };

        let app_state = match app_state {
            Some(s) => s,
            None => {
                return Box::pin(async move {
                    Err(actix_web::Error::from(AppError::internal(
                        crate::errors::ErrorCode::InternalError,
                        "AppState not available",
                        std::io::Error::other("AppState missing"),
                    )))
                });
            }
        };

        let mut conn = match app_state.session_redis() {
            Some(c) => c,
            None => {
                return Box::pin(async move {
                    Err(actix_web::Error::from(AppError::redis_unavailable(
                        "session Redis not available".to_string(),
                        crate::error::Sentinel("session Redis not available"),
                        None,
                    )))
                });
            }
        };

        Box::pin(async move {
            let result = if is_ws_token {
                get_ws_token(&mut conn, &token).await
            } else {
                get_and_slide_session(&mut conn, &token).await
            };

            match result {
                Ok(Some(session_data)) => {
                    req.extensions_mut().insert(session_data);
                    service.call(req).await
                }
                Ok(None) => Err(actix_web::Error::from(
                    AppError::unauthorized_invalid_token(),
                )),
                Err(e) => Err(actix_web::Error::from(e)),
            }
        })
    }
}

fn extract_token_from_query(query: Option<&str>) -> Option<String> {
    let query_str = query?;
    let params = web::Query::<HashMap<String, String>>::from_query(query_str).ok()?;
    params.get("token").cloned().filter(|v| !v.is_empty())
}
