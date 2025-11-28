use jsonwebtoken::Algorithm;

/// Configuration for JWT security settings
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// JWT secret key for signing and verifying tokens
    pub jwt_secret: Vec<u8>,
    /// JWT algorithm to use (defaults to HS256)
    pub algorithm: Algorithm,
    /// Optional JWT issuer claim
    pub issuer: Option<String>,
    /// Optional JWT audience claim
    pub audience: Option<String>,
}

impl SecurityConfig {
    /// Create a new SecurityConfig with the given JWT secret
    pub fn new(jwt_secret: impl Into<Vec<u8>>) -> Self {
        Self {
            jwt_secret: jwt_secret.into(),
            algorithm: Algorithm::HS256,
            issuer: None,
            audience: None,
        }
    }

    /// Create a new SecurityConfig with custom algorithm
    pub fn with_algorithm(mut self, algorithm: Algorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Set the JWT issuer claim
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Set the JWT audience claim
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    /// Create a test configuration with a random secret
    #[cfg(test)]
    pub fn for_tests() -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        let secret: [u8; 32] = rng.random();
        Self::new(secret.to_vec())
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self::new(b"default_secret_for_tests_only".to_vec())
    }
}
