//! Backend-specific JWT claims used across the application.

use serde::{Deserialize, Serialize};

/// Backend-specific JWT claims structure inserted into request extensions
/// by the authentication middleware.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackendClaims {
    /// External user identifier (users.sub)
    pub sub: String,
    pub email: String,
    /// Expiry (seconds since epoch)
    pub exp: usize,
}
