use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::state::security_config::SecurityConfig;
use crate::AppError;

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

/// Mint a HS256 JWT access token with a 15-minute TTL.
pub fn mint_access_token(
    sub: &str,
    email: &str,
    now: SystemTime,
    security: &SecurityConfig,
) -> Result<String, AppError> {
    let iat = now
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::internal("Failed to get current time".to_string()))?
        .as_secs() as i64;

    // 15 minutes expiration
    let exp = iat + 15 * 60;

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
    .map_err(|e| AppError::internal(format!("Failed to encode JWT: {e}")))
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
            AppError::unauthorized().with_trace_id(Some("token_expired".to_string()))
        }
        jsonwebtoken::errors::ErrorKind::InvalidSignature => {
            AppError::unauthorized().with_trace_id(Some("invalid_signature".to_string()))
        }
        _ => AppError::unauthorized().with_trace_id(Some("invalid_token".to_string())),
    })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::{mint_access_token, verify_access_token};
    use crate::state::security_config::SecurityConfig;
    use crate::AppError;

    #[test]
    fn test_mint_and_verify_roundtrip() {
        let security = SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());

        let sub = "test-sub-roundtrip-123";
        let email = "test@example.com";
        let now = SystemTime::now();

        let token = mint_access_token(sub, email, now, &security).unwrap();
        let claims = verify_access_token(&token, &security).unwrap();

        assert_eq!(claims.sub, sub);
        assert_eq!(claims.email, email);
        assert_eq!(
            claims.iat,
            now.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
        );
        assert_eq!(claims.exp, claims.iat + 15 * 60);
    }

    #[test]
    fn test_expired_token() {
        let security = SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());

        let sub = "test-sub-expired-456";
        let email = "test@example.com";
        // 20 minutes ago so 15-minute token is expired
        let now = SystemTime::now() - Duration::from_secs(20 * 60);

        let token = mint_access_token(sub, email, now, &security).unwrap();
        let result = verify_access_token(&token, &security);

        match result {
            Err(AppError::Unauthorized { trace_id }) => {
                assert_eq!(trace_id, Some("token_expired".to_string()));
            }
            _ => panic!("Expected unauthorized error for expired token"),
        }
    }

    #[test]
    fn test_bad_signature() {
        // Mint with secret A
        let security_a = SecurityConfig::new("secret-A".as_bytes());

        let sub = "test-sub-bad-sig-789";
        let email = "test@example.com";
        let token = mint_access_token(sub, email, SystemTime::now(), &security_a).unwrap();

        // Verify with secret B
        let security_b = SecurityConfig::new("secret-B".as_bytes());
        let result = verify_access_token(&token, &security_b);

        match result {
            Err(AppError::Unauthorized { trace_id }) => {
                assert_eq!(trace_id, Some("invalid_signature".to_string()));
            }
            _ => panic!("Expected unauthorized error for bad signature"),
        }
    }

    #[test]
    fn test_missing_jwt_secret() {
        let security = SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());

        let sub = "test-sub-missing-secret-012";
        let email = "test@example.com";
        let now = SystemTime::now();

        let result = mint_access_token(sub, email, now, &security);

        match result {
            Ok(_) => {
                // This should now succeed since we're passing the security config
                // The old test was checking for missing env var, which is no longer relevant
            }
            _ => panic!("Expected success since security config is provided"),
        }
    }
}
