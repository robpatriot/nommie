use jsonwebtoken::Algorithm;

/// Configuration for JWT security settings
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// JWT secret key for signing and verifying tokens
    pub jwt_secret: Vec<u8>,
    /// JWT algorithm to use (defaults to HS256)
    pub algorithm: Algorithm,
}

impl SecurityConfig {
    /// Create a new SecurityConfig with the given JWT secret
    pub fn new(jwt_secret: impl Into<Vec<u8>>) -> Self {
        Self {
            jwt_secret: jwt_secret.into(),
            algorithm: Algorithm::HS256,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self::new(b"default_secret_for_tests_only".to_vec())
    }
}
