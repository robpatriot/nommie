//! JWT extraction middleware
//!
//! This middleware extracts JWT claims from the Authorization header and stores them
//! in request extensions. It only runs on protected routes (/api/games/*)
//! and returns 401 if no valid claims are found.

use std::collections::HashMap;

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header;
use actix_web::{web, Error, HttpMessage};
use futures_util::future::{ready, LocalBoxFuture, Ready};

use crate::auth::claims::BackendClaims;
use crate::auth::jwt::JwtClaims;
use crate::error::AppError;
use crate::state::app_state::AppState;

pub struct JwtExtract;

impl<S, B> Transform<S, ServiceRequest> for JwtExtract
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtExtractMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtExtractMiddleware { service }))
    }
}

pub struct JwtExtractMiddleware<S> {
    service: S,
}

fn unauthorized_error(detail: String) -> Error {
    actix_web::error::ErrorUnauthorized(detail)
}

impl<S, B> Service<ServiceRequest> for JwtExtractMiddleware<S>
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
        // Extract Authorization sources and AppState before moving req
        let auth_header = req.headers().get(header::AUTHORIZATION).cloned();
        let app_state = req.app_data::<web::Data<AppState>>().cloned();

        // Parse token from Authorization header or query string fallback (for WebSocket)
        let token = match extract_bearer_from_header(auth_header.as_ref()) {
            Ok(Some(token)) => token,
            Ok(None) => match extract_token_from_query(req.uri().query()) {
                Some(token) => token,
                None => {
                    return Box::pin(async {
                        Err(unauthorized_error(
                            "Missing Authorization header".to_string(),
                        ))
                    })
                }
            },
            Err(err) => {
                return Box::pin(async { Err(err) });
            }
        };

        // Get AppState - must be available
        let app_state = match app_state {
            Some(state) => state,
            None => {
                return Box::pin(async {
                    Err(actix_web::error::ErrorInternalServerError(
                        "AppState not available",
                    ))
                });
            }
        };

        // Verify the JWT token
        let jwt_result = JwtClaims::<BackendClaims>::verify(&token, &app_state.security);

        match jwt_result {
            Ok(jwt_claims) => {
                // Check email allowlist if enabled (invalidates existing sessions for non-allowlisted users)
                if let Some(allowlist) = &app_state.email_allowlist {
                    let email = &jwt_claims.claims.email;
                    if !allowlist.is_allowed(email) {
                        // Use structured AppError so the frontend receives a proper
                        // Problem Details response with code=EMAIL_NOT_ALLOWED.
                        return Box::pin(async { Err(AppError::email_not_allowed().into()) });
                    }
                }

                // Store claims in request extensions BEFORE calling the service
                req.extensions_mut().insert(jwt_claims.claims);

                // Call the downstream service and get the future
                let fut = self.service.call(req);
                Box::pin(fut)
            }
            Err(e) => Box::pin(async move { Err(unauthorized_error(format!("{}", e))) }),
        }
    }
}

fn extract_bearer_from_header(
    header_value: Option<&actix_web::http::HeaderValue>,
) -> Result<Option<String>, Error> {
    let auth_value = match header_value {
        Some(value) => value,
        None => return Ok(None),
    };

    let auth_str = match auth_value.to_str() {
        Ok(s) => s,
        Err(_) => {
            return Err(unauthorized_error(
                "Missing or invalid Authorization header".to_string(),
            ))
        }
    };

    let parts: Vec<&str> = auth_str.split_whitespace().collect();
    if parts.len() != 2 || parts[0] != "Bearer" {
        return Err(unauthorized_error(
            "Missing or invalid Bearer token".to_string(),
        ));
    }

    let token_str = parts[1];
    if token_str.is_empty() {
        return Err(unauthorized_error(
            "Missing or invalid Bearer token".to_string(),
        ));
    }

    Ok(Some(token_str.to_string()))
}

fn extract_token_from_query(query: Option<&str>) -> Option<String> {
    let query_str = query?;
    let params = web::Query::<HashMap<String, String>>::from_query(query_str).ok()?;
    params
        .get("token")
        .cloned()
        .filter(|value| !value.is_empty())
}
