//! JWT extraction middleware
//!
//! This middleware extracts JWT claims from the Authorization header and stores them
//! in request extensions. It only runs on protected routes (/api/games/*)
//! and returns 401 if no valid claims are found.

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header;
use actix_web::{web, Error, HttpMessage};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use tracing::warn;

use crate::auth::claims::BackendClaims;
use crate::auth::jwt::JwtClaims;
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
        // [AUTH_BYPASS] START - Temporary debugging feature - remove when done
        // Check if authentication bypass is enabled via environment variable
        let disable_auth = std::env::var("DISABLE_AUTH")
            .unwrap_or_default()
            .parse::<bool>()
            .unwrap_or(false);

        if disable_auth {
            warn!("⚠️  AUTHENTICATION BYPASSED in JWT middleware - This should only be enabled for debugging!");

            // Get test user configuration from environment
            let test_sub =
                std::env::var("TEST_USER_SUB").unwrap_or_else(|_| "test-user".to_string());
            let test_email =
                std::env::var("TEST_USER_EMAIL").unwrap_or_else(|_| "test@example.com".to_string());

            // Create fake BackendClaims for bypass mode
            let fake_claims = BackendClaims {
                sub: test_sub,
                email: test_email,
                exp: usize::MAX, // Never expires
            };

            // Store claims in request extensions
            req.extensions_mut().insert(fake_claims);

            // Call the downstream service without JWT validation
            let fut = self.service.call(req);
            return Box::pin(fut);
        }
        // [AUTH_BYPASS] END

        // Extract Authorization header and AppState before moving req
        let auth_header = req.headers().get(header::AUTHORIZATION).cloned();
        let app_state = req.app_data::<web::Data<AppState>>().cloned();

        // Parse token from Authorization header (check early for quick failure)
        let token = if let Some(auth_value) = auth_header.as_ref() {
            let auth_str = match auth_value.to_str() {
                Ok(s) => s,
                Err(_) => {
                    return Box::pin(async {
                        Err(unauthorized_error(
                            "Missing or invalid Authorization header".to_string(),
                        ))
                    });
                }
            };

            // Parse "Bearer <token>" format
            let parts: Vec<&str> = auth_str.split_whitespace().collect();
            if parts.len() != 2 || parts[0] != "Bearer" {
                return Box::pin(async {
                    Err(unauthorized_error(
                        "Missing or invalid Bearer token".to_string(),
                    ))
                });
            }

            let token_str = parts[1];
            if token_str.is_empty() {
                return Box::pin(async {
                    Err(unauthorized_error(
                        "Missing or invalid Bearer token".to_string(),
                    ))
                });
            }

            token_str.to_string()
        } else {
            return Box::pin(async {
                Err(unauthorized_error(
                    "Missing Authorization header".to_string(),
                ))
            });
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
