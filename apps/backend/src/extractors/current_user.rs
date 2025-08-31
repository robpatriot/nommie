use actix_web::{dev::Payload, FromRequest, HttpRequest};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::jwt::JwtClaims;

/// Backend-specific JWT claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackendClaims {
    /// Our internal user id
    pub sub: Uuid,
    pub email: String,
    /// Expiry (seconds since epoch)
    pub exp: usize,
}

/// Current user extracted from a valid JWT token
/// This is a thin wrapper around JwtClaims<BackendClaims>
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurrentUser {
    pub sub: Uuid,
    pub email: String,
}

impl From<JwtClaims<BackendClaims>> for CurrentUser {
    fn from(jwt_claims: JwtClaims<BackendClaims>) -> Self {
        CurrentUser {
            sub: jwt_claims.claims.sub,
            email: jwt_claims.claims.email,
        }
    }
}

impl From<BackendClaims> for CurrentUser {
    fn from(claims: BackendClaims) -> Self {
        CurrentUser {
            sub: claims.sub,
            email: claims.email,
        }
    }
}

impl FromRequest for CurrentUser {
    type Error = crate::AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();

        Box::pin(async move {
            // Extract JwtClaims<BackendClaims> first
            let jwt_claims = JwtClaims::<BackendClaims>::from_request(&req, &mut payload).await?;

            // Convert to CurrentUser
            Ok(CurrentUser::from(jwt_claims))
        })
    }
}
