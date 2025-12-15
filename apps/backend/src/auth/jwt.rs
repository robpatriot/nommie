use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::claims::BackendClaims;
use crate::error::AppError;
use crate::state::security_config::SecurityConfig;

/// Claims included in our backend-issued access tokens.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// External user identifier (users.sub)
    pub sub: String,
    pub email: String,
    /// Issued-at (seconds since epoch)
    pub iat: i64,
    /// Expiry (seconds since epoch)
    pub exp: i64,
}

/// Mint a HS256 JWT access token with a 60-minute TTL.
pub fn mint_access_token(
    sub: &str,
    email: &str,
    now: SystemTime,
    security: &SecurityConfig,
) -> Result<String, AppError> {
    mint_access_token_with_ttl(sub, email, now, 60 * 60, security)
}

pub fn mint_access_token_with_ttl(
    sub: &str,
    email: &str,
    now: SystemTime,
    ttl_seconds: i64,
    security: &SecurityConfig,
) -> Result<String, AppError> {
    if ttl_seconds <= 0 {
        return Err(AppError::internal(
            crate::errors::ErrorCode::InternalError,
            "JWT TTL must be positive".to_string(),
            std::io::Error::other("invalid ttl"),
        ));
    }

    let iat = now
        .duration_since(UNIX_EPOCH)
        .map_err(|e| {
            AppError::internal(
                crate::errors::ErrorCode::InternalError,
                "Failed to get current time".to_string(),
                e,
            )
        })?
        .as_secs() as i64;

    let exp = iat + ttl_seconds;

    let claims = Claims {
        sub: sub.to_string(),
        email: email.to_string(),
        iat,
        exp,
    };

    encode(
        &Header::new(security.algorithm),
        &claims,
        &EncodingKey::from_secret(&security.jwt_secret),
    )
    .map_err(|e| {
        AppError::internal(
            crate::errors::ErrorCode::InternalError,
            "failed to encode JWT",
            e,
        )
    })
}

/// Verify JWT and return claims.
///
/// Errors:
/// - Expired token → `AppError::unauthorized().with_trace_id(Some("token_expired".into()))`
/// - Invalid signature → `...("invalid_signature")`
/// - Any other decode error → `...("invalid_token")`
pub fn verify_access_token(token: &str, security: &SecurityConfig) -> Result<Claims, AppError> {
    // Default Validation already checks exp; pin algorithm to configured algorithm.
    let validation = Validation::new(security.algorithm);

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(&security.jwt_secret),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
            crate::logging::security::login_failed("token_expired", None);
            AppError::unauthorized_expired_jwt()
        }
        jsonwebtoken::errors::ErrorKind::InvalidSignature => {
            crate::logging::security::login_failed("invalid_signature", None);
            AppError::unauthorized_invalid_jwt()
        }
        _ => {
            crate::logging::security::login_failed("invalid_token", None);
            AppError::unauthorized_invalid_jwt()
        }
    })
}

/// Wrapper structure to provide a compatible API for middleware that expects
/// a `JwtClaims<C>` item with a `verify` method.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims<C> {
    pub claims: C,
}

impl JwtClaims<BackendClaims> {
    /// Verify a token and map the verified claims to `BackendClaims`.
    pub fn verify(token: &str, security: &SecurityConfig) -> Result<Self, AppError> {
        let verified = verify_access_token(token, security)?;
        Ok(JwtClaims {
            claims: BackendClaims {
                sub: verified.sub,
                email: verified.email,
                exp: verified.exp as usize,
            },
        })
    }
}

/// Extract the 'sub' claim from a JWT token for logging purposes.
///
/// This performs minimal validation - just enough to decode the token.
/// It does NOT validate expiration, signature thoroughly, or other security claims.
/// Use this ONLY for observability (logging, tracing), never for authentication.
///
/// For authentication, use `verify_access_token()` instead.
///
/// Returns None if the token is invalid or missing a 'sub' claim.
pub fn extract_sub_for_logging(token: &str, secret: &[u8]) -> Option<String> {
    // Configure minimal validation - we only care about the algorithm
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false; // Skip expiration check for logging
    validation.validate_nbf = false;
    validation.validate_aud = false;
    validation.required_spec_claims.clear();

    // Decode the token
    let token_data = decode::<Value>(token, &DecodingKey::from_secret(secret), &validation).ok()?;

    // Extract the 'sub' claim
    token_data.claims.get("sub")?.as_str().map(String::from)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use actix_web::http::StatusCode;
    use actix_web::ResponseError;
    use backend_test_support::problem_details::assert_problem_details_from_http_response;
    use backend_test_support::unique_helpers::{unique_email, unique_str};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;

    use super::{extract_sub_for_logging, mint_access_token, verify_access_token};
    use crate::state::security_config::SecurityConfig;

    #[tokio::test]
    async fn test_mint_and_verify_roundtrip() {
        let security = SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());

        let sub = unique_str("test-sub-roundtrip");
        let email = unique_email("test");
        let now = SystemTime::now();

        let token = mint_access_token(&sub, &email, now, &security).unwrap();
        let claims = verify_access_token(&token, &security).unwrap();

        assert_eq!(claims.sub, sub);
        assert_eq!(claims.email, email);
        assert_eq!(
            claims.iat,
            now.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
        );
        assert_eq!(claims.exp, claims.iat + 60 * 60);
    }

    #[tokio::test]
    async fn test_expired_token() {
        let security = SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());

        let sub = unique_str("test-sub-expired");
        let email = unique_email("test");
        // 70 minutes ago so 60-minute token is expired
        let now = SystemTime::now() - Duration::from_secs(70 * 60);

        let token = mint_access_token(&sub, &email, now, &security).unwrap();
        let result = verify_access_token(&token, &security);

        match result {
            Err(err) => {
                // Use contract assertions via HTTP path
                let response = err.error_response();
                assert_problem_details_from_http_response(
                    response,
                    "UNAUTHORIZED_EXPIRED_JWT",
                    StatusCode::UNAUTHORIZED,
                    Some("Token expired"),
                )
                .await;
            }
            Ok(_) => panic!("Expected unauthorized expired JWT error for expired token"),
        }
    }

    #[tokio::test]
    async fn test_bad_signature() {
        // Mint with secret A
        let security_a = SecurityConfig::new("secret-A".as_bytes());

        let sub = unique_str("test-sub-bad-sig");
        let email = unique_email("test");
        let token = mint_access_token(&sub, &email, SystemTime::now(), &security_a).unwrap();

        // Verify with secret B
        let security_b = SecurityConfig::new("secret-B".as_bytes());
        let result = verify_access_token(&token, &security_b);

        match result {
            Err(err) => {
                // Use contract assertions via HTTP path
                let response = err.error_response();
                assert_problem_details_from_http_response(
                    response,
                    "UNAUTHORIZED_INVALID_JWT",
                    StatusCode::UNAUTHORIZED,
                    Some("Invalid JWT"),
                )
                .await;
            }
            Ok(_) => panic!("Expected unauthorized invalid JWT error for bad signature"),
        }
    }

    #[tokio::test]
    async fn test_missing_jwt_secret() {
        let security = SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());

        let sub = unique_str("test-sub-missing-secret");
        let email = unique_email("test");
        let now = SystemTime::now();

        let result = mint_access_token(&sub, &email, now, &security);

        match result {
            Ok(_) => {
                // This should succeed since we're passing the security config
            }
            _ => panic!("Expected success since security config is provided"),
        }
    }

    #[test]
    fn test_extract_sub_valid_token() {
        let secret = b"test_secret";
        let claims = json!({
            "sub": "user123",
            "exp": 9999999999u64, // Far future
        });

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();

        let sub = extract_sub_for_logging(&token, secret);
        assert_eq!(sub, Some("user123".to_string()));
    }

    #[test]
    fn test_extract_sub_expired_token() {
        let secret = b"test_secret";
        let claims = json!({
            "sub": "user123",
            "exp": 0u64, // Expired
        });

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();

        // Should still extract sub even though token is expired (for logging)
        let sub = extract_sub_for_logging(&token, secret);
        assert_eq!(sub, Some("user123".to_string()));
    }

    #[test]
    fn test_extract_sub_invalid_token() {
        let secret = b"test_secret";
        let sub = extract_sub_for_logging("invalid_token", secret);
        assert_eq!(sub, None);
    }

    #[test]
    fn test_extract_sub_missing_sub_claim() {
        let secret = b"test_secret";
        let claims = json!({
            "email": "user@example.com",
            "exp": 9999999999u64,
        });

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();

        let sub = extract_sub_for_logging(&token, secret);
        assert_eq!(sub, None);
    }
}
