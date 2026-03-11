//! AdminPrincipal extractor – admin boundary check for /api/admin.

use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};

use crate::authz::{has_capability, AdminCapability, Principal};
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::extractors::current_user::CurrentUser;

/// Authenticated admin principal. Handlers under /api/admin take this extractor.
/// Enforces AccessAdmin at the boundary; fails with 403 if the caller lacks it.
#[derive(Debug, Clone)]
pub struct AdminPrincipal(pub Principal);

impl std::ops::Deref for AdminPrincipal {
    type Target = Principal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for AdminPrincipal {
    type Error = AppError;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let fut = CurrentUser::from_request(req, payload);
        Box::pin(async move {
            let current_user = fut.await?;
            let principal = Principal {
                user_id: current_user.id,
                role: current_user.role,
            };
            if !has_capability(&principal, AdminCapability::AccessAdmin) {
                return Err(AppError::forbidden_with_code(
                    ErrorCode::PermissionDenied,
                    "Admin access required",
                ));
            }
            Ok(AdminPrincipal(principal))
        })
    }
}
