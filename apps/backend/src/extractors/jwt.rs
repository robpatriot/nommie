use actix_web::dev::Payload;
use actix_web::{web, FromRequest, HttpRequest};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::state::app_state::AppState;
use crate::error::AppError;

/// Generic JWT claims that can be validated against any claims type
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims<C> {
    pub claims: C,
}

impl<C> JwtClaims<C>
where
    C: for<'de> Deserialize<'de>,
{
    /// Verify and decode a JWT token into the specified claims type
    pub fn verify(
        token: &str,
        security: &crate::state::security_config::SecurityConfig,
    ) -> Result<Self, AppError> {
        // Configure validation to check expiration and pin algorithm to configured algorithm.
        let mut validation = Validation::new(security.algorithm);
        validation.validate_exp = true;

        decode::<C>(
            token,
            &DecodingKey::from_secret(&security.jwt_secret),
            &validation,
        )
        .map(|data| JwtClaims {
            claims: data.claims,
        })
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                AppError::unauthorized_expired_jwt()
            }
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                AppError::unauthorized_invalid_jwt()
            }
            _ => AppError::unauthorized_invalid_jwt(),
        })
    }

    /// Verify and decode a JWT token into the specified claims type with request context
    pub fn verify_with_request(
        token: &str,
        _req: &HttpRequest,
        security: &crate::state::security_config::SecurityConfig,
    ) -> Result<Self, AppError> {
        // Configure validation to check expiration and pin algorithm to configured algorithm.
        let mut validation = Validation::new(security.algorithm);
        validation.validate_exp = true;

        decode::<C>(
            token,
            &DecodingKey::from_secret(&security.jwt_secret),
            &validation,
        )
        .map(|data| JwtClaims {
            claims: data.claims,
        })
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                AppError::unauthorized_expired_jwt()
            }
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                AppError::unauthorized_invalid_jwt()
            }
            _ => AppError::unauthorized_invalid_jwt(),
        })
    }
}

impl<C> FromRequest for JwtClaims<C>
where
    C: for<'de> Deserialize<'de> + 'static,
{
    type Error = AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // Extract Authorization header
            let auth_header = req
                .headers()
                .get(actix_web::http::header::AUTHORIZATION)
                .ok_or(AppError::unauthorized_missing_bearer())?;

            let auth_value = auth_header
                .to_str()
                .map_err(|_| AppError::unauthorized_missing_bearer())?;

            // Parse "Bearer <token>" format
            let parts: Vec<&str> = auth_value.split_whitespace().collect();
            if parts.len() != 2 || parts[0] != "Bearer" {
                return Err(AppError::unauthorized_missing_bearer());
            }

            let token = parts[1];
            if token.is_empty() {
                return Err(AppError::unauthorized_missing_bearer());
            }

            // Get the security config from the request data
            let app_state = req
                .app_data::<web::Data<AppState>>()
                .ok_or_else(|| AppError::internal("AppState not found"))?;

            // Verify the JWT token
            JwtClaims::verify(token, &app_state.security)
        })
    }
}
