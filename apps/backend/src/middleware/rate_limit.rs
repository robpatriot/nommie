//! Rate limiting middleware configuration helpers
//!
//! Provides configuration for different rate limit settings:
//! - Authentication endpoints: 5 requests per minute per IP
//! - General API endpoints: 100 requests per minute per IP
//! - Health check: Exempt from rate limiting

use std::time::Duration;

use actix_extensible_rate_limit::backend::SimpleInputFunctionBuilder;

/// Configuration for authentication endpoint rate limiting.
/// Limits: 5 requests per 60 seconds per IP address.
pub fn auth_rate_limit_config() -> SimpleInputFunctionBuilder {
    SimpleInputFunctionBuilder::new(Duration::from_secs(60), 5).real_ip_key()
}

/// Configuration for general API endpoint rate limiting.
/// Limits: 100 requests per 60 seconds per IP address.
pub fn api_rate_limit_config() -> SimpleInputFunctionBuilder {
    SimpleInputFunctionBuilder::new(Duration::from_secs(60), 100).real_ip_key()
}
