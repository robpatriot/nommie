//! Security headers middleware
//!
//! Adds security headers to all responses:
//! - X-Content-Type-Options: nosniff
//! - X-Frame-Options: DENY
//! - Strict-Transport-Security: max-age=31536000; includeSubDomains
//! - Referrer-Policy: strict-origin-when-cross-origin
//! - Content-Security-Policy: Restrictive policy to prevent XSS attacks
//! - Permissions-Policy: Restricts browser features
//! - Cache-Control: no-store (only for /api/* and /health endpoints)

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header;
use actix_web::Error as ActixError;
use futures_util::future::{ready, LocalBoxFuture, Ready};

pub struct SecurityHeaders;

impl<S, B> Transform<S, ServiceRequest> for SecurityHeaders
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type InitError = ();
    type Transform = SecurityHeadersMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SecurityHeadersMiddleware { service }))
    }
}

pub struct SecurityHeadersMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let path = req.path().to_string();
        let fut = self.service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;

            // Add security headers to the response
            let headers = res.headers_mut();

            // X-Content-Type-Options: nosniff - Prevents MIME sniffing
            headers.insert(
                header::HeaderName::from_static("x-content-type-options"),
                header::HeaderValue::from_static("nosniff"),
            );

            // X-Frame-Options: DENY - Prevents clickjacking
            headers.insert(
                header::HeaderName::from_static("x-frame-options"),
                header::HeaderValue::from_static("DENY"),
            );

            // Strict-Transport-Security: max-age=31536000; includeSubDomains
            // Browsers will only honor this on HTTPS connections, so safe to always set
            headers.insert(
                header::HeaderName::from_static("strict-transport-security"),
                header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
            );

            // Referrer-Policy: strict-origin-when-cross-origin
            headers.insert(
                header::HeaderName::from_static("referrer-policy"),
                header::HeaderValue::from_static("strict-origin-when-cross-origin"),
            );

            // Content-Security-Policy: Restrictive policy to prevent XSS attacks
            // For API endpoints, we can be very restrictive since we only return JSON
            if path.starts_with("/api/") || path == "/health" {
                // API endpoints: very restrictive CSP (no scripts, styles, etc.)
                headers.insert(
                    header::HeaderName::from_static("content-security-policy"),
                    header::HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
                );
            } else {
                // For other endpoints (like root), use a more permissive CSP
                // This can be customized per route if needed
                headers.insert(
                    header::HeaderName::from_static("content-security-policy"),
                    header::HeaderValue::from_static("default-src 'self'; frame-ancestors 'none'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self' https:;"),
                );
            }

            // Permissions-Policy: Restrict browser features
            // Disable geolocation, microphone, camera, and other sensitive features
            headers.insert(
                header::HeaderName::from_static("permissions-policy"),
                header::HeaderValue::from_static("geolocation=(), microphone=(), camera=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()"),
            );

            // X-XSS-Protection: Included for older browser support
            headers.insert(
                header::HeaderName::from_static("x-xss-protection"),
                header::HeaderValue::from_static("1; mode=block"),
            );

            // Cache-Control: no-store - Only for API and health endpoints
            // Skip for root / endpoint to allow browser default caching behavior
            if path.starts_with("/api/") || path == "/health" {
                headers.insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("no-store"),
                );
            }

            Ok(res)
        })
    }
}
