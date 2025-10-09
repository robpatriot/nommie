//! JWT token generation helpers for tests

use std::time::SystemTime;

use backend::auth::jwt::mint_access_token;
use backend::state::security_config::SecurityConfig;

/// Mint a bearer token for the given sub and email
///
/// # Arguments
/// * `sub` - Subject identifier (user's sub)
/// * `email` - User's email
/// * `sec` - Security configuration containing JWT secret
///
/// # Returns
/// Bearer token string (without "Bearer " prefix)
pub fn mint_test_token(sub: &str, email: &str, sec: &SecurityConfig) -> String {
    mint_access_token(sub, email, SystemTime::now(), sec).expect("should mint token successfully")
}

/// Mint a bearer Authorization header value for the given sub and email
///
/// # Arguments
/// * `sub` - Subject identifier (user's sub)
/// * `email` - User's email
/// * `sec` - Security configuration containing JWT secret
///
/// # Returns
/// Full Authorization header value including "Bearer " prefix
pub fn bearer_header(sub: &str, email: &str, sec: &SecurityConfig) -> String {
    format!("Bearer {}", mint_test_token(sub, email, sec))
}

/// Mint an expired token for testing expired token scenarios
///
/// # Arguments
/// * `sub` - Subject identifier (user's sub)
/// * `email` - User's email
/// * `sec` - Security configuration containing JWT secret
///
/// # Returns
/// Expired bearer token string
pub fn mint_expired_token(sub: &str, email: &str, sec: &SecurityConfig) -> String {
    let past_time = SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(7200))
        .unwrap();
    mint_access_token(sub, email, past_time, sec).expect("should mint expired token successfully")
}
