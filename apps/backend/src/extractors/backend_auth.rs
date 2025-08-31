use actix_web::{dev::Payload, http::header, FromRequest, HttpRequest};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::verify_access_token, AppError};

/// Authentication data extracted from a valid JWT token
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackendAuth {
    pub sub: Uuid,
    pub email: String,
}

impl FromRequest for BackendAuth {
    type Error = AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // Extract Authorization header
            let auth_header = req
                .headers()
                .get(header::AUTHORIZATION)
                .ok_or_else(|| AppError::from_req(&req, AppError::unauthorized()))?;

            let auth_value = auth_header
                .to_str()
                .map_err(|_| AppError::from_req(&req, AppError::unauthorized()))?;

            // Parse "Bearer <token>" format
            let parts: Vec<&str> = auth_value.split_whitespace().collect();
            if parts.len() != 2 || parts[0] != "Bearer" {
                return Err(AppError::from_req(&req, AppError::unauthorized()));
            }

            let token = parts[1];
            if token.is_empty() {
                return Err(AppError::from_req(&req, AppError::unauthorized()));
            }

            // Verify the JWT token
            let claims = verify_access_token(token).map_err(|e| AppError::from_req(&req, e))?;

            // Map to BackendAuth
            Ok(BackendAuth {
                sub: claims.sub,
                email: claims.email,
            })
        })
    }
}
