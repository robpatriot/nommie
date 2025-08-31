use actix_web::{dev::Payload, http::header, FromRequest, HttpRequest};
use serde::{Deserialize, Serialize};

use crate::AppError;

/// Authentication token extracted from the Authorization header
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthToken {
    pub token: String,
}

impl FromRequest for AuthToken {
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

            Ok(AuthToken {
                token: token.to_string(),
            })
        })
    }
}
