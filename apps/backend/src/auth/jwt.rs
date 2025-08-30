use crate::AppError;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Claims included in our backend-issued access tokens.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Our internal user id
    pub sub: Uuid,
    pub email: String,
    /// Issued-at (seconds since epoch)
    pub iat: i64,
    /// Expiry (seconds since epoch)
    pub exp: i64,
}

/// Mint a HS256 JWT access token with a 15-minute TTL.
pub fn mint_access_token(user_id: Uuid, email: &str, now: SystemTime) -> Result<String, AppError> {
    let secret = env::var("APP_JWT_SECRET")
        .map_err(|_| AppError::config("Missing APP_JWT_SECRET environment variable".to_string()))?;

    let iat = now
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::internal("Failed to get current time".to_string()))?
        .as_secs() as i64;

    // 15 minutes expiration
    let exp = iat + 15 * 60;

    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        iat,
        exp,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(|e| AppError::internal(format!("Failed to encode JWT: {e}")))
}

/// Verify HS256 JWT and return claims.
///
/// Errors:
/// - Missing secret → `AppError::config(...)`
/// - Expired token → `AppError::unauthorized().with_trace_id(Some("token_expired".into()))`
/// - Invalid signature → `...("invalid_signature")`
/// - Any other decode error → `...("invalid_token")`
pub fn verify_access_token(token: &str) -> Result<Claims, AppError> {
    let secret = env::var("APP_JWT_SECRET")
        .map_err(|_| AppError::config("Missing APP_JWT_SECRET environment variable".to_string()))?;

    // Default Validation already checks exp; pin algorithm to HS256.
    let validation = Validation::new(Algorithm::HS256);

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
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
    use super::*;
    use serial_test::serial;
    use std::time::Duration;

    fn set_secret(val: &str) -> Option<String> {
        let original = env::var("APP_JWT_SECRET").ok();
        env::set_var("APP_JWT_SECRET", val);
        original
    }

    fn restore_secret(original: Option<String>) {
        if let Some(v) = original {
            env::set_var("APP_JWT_SECRET", v);
        } else {
            env::remove_var("APP_JWT_SECRET");
        }
    }

    #[test]
    #[serial]
    fn test_mint_and_verify_roundtrip() {
        let original = set_secret("test_secret_key_for_testing_purposes_only");

        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let now = SystemTime::now();

        let token = mint_access_token(user_id, email, now).unwrap();
        let claims = verify_access_token(&token).unwrap();

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(
            claims.iat,
            now.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
        );
        assert_eq!(claims.exp, claims.iat + 15 * 60);

        restore_secret(original);
    }

    #[test]
    #[serial]
    fn test_expired_token() {
        let original = set_secret("test_secret_key_for_testing_purposes_only");

        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        // 20 minutes ago so 15-minute token is expired
        let now = SystemTime::now() - Duration::from_secs(20 * 60);

        let token = mint_access_token(user_id, email, now).unwrap();
        let result = verify_access_token(&token);

        match result {
            Err(AppError::Unauthorized { trace_id }) => {
                assert_eq!(trace_id, Some("token_expired".to_string()));
            }
            _ => panic!("Expected unauthorized error for expired token"),
        }

        restore_secret(original);
    }

    #[test]
    #[serial]
    fn test_bad_signature() {
        // Mint with secret A
        let original = set_secret("secret-A");

        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let token = mint_access_token(user_id, email, SystemTime::now()).unwrap();

        // Verify with secret B
        env::set_var("APP_JWT_SECRET", "secret-B");
        let result = verify_access_token(&token);

        match result {
            Err(AppError::Unauthorized { trace_id }) => {
                assert_eq!(trace_id, Some("invalid_signature".to_string()));
            }
            _ => panic!("Expected unauthorized error for bad signature"),
        }

        restore_secret(original);
    }

    #[test]
    #[serial]
    fn test_missing_jwt_secret() {
        let original = env::var("APP_JWT_SECRET").ok();
        env::remove_var("APP_JWT_SECRET");

        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let now = SystemTime::now();

        let result = mint_access_token(user_id, email, now);

        // Restore original secret first to not affect other tests
        restore_secret(original);

        match result {
            Err(AppError::Config { detail, .. }) => {
                assert_eq!(detail, "Missing APP_JWT_SECRET environment variable");
            }
            _ => panic!("Expected config error for missing JWT secret"),
        }
    }
}
