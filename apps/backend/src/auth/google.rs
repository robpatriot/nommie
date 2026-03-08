//! Google ID token verification for backend authentication.
//!
//! The backend verifies Google ID tokens server-side using OpenID Connect
//! discovery and JWKS, deriving trusted identity claims. Client-posted
//! identity fields are never trusted.
//!
//! Supports Google signing key rotation: on `NoMatchingKey` (key ID not in
//! cached JWKS), the verifier re-fetches provider metadata and JWKS once,
//! then retries. Only one refresh runs at a time; other requests wait and
//! retry with the updated keys.

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use openidconnect::core::{CoreClient, CoreIdToken, CoreProviderMetadata};
use openidconnect::{
    ClaimsVerificationError, ClientId, IssuerUrl, NonceVerifier, SignatureVerificationError,
};

use crate::error::AppError;
use crate::errors::ErrorCode;

/// Trusted claims extracted from a verified Google ID token.
#[derive(Debug, Clone)]
pub struct VerifiedGoogleClaims {
    /// Google's unique user ID (provider_user_id for user_auth_identities)
    pub sub: String,
    /// Verified email address
    pub email: String,
    /// Display name (optional)
    pub name: Option<String>,
}

/// Abstraction for verifying Google ID tokens.
/// Allows tests to inject a mock without calling live Google.
#[async_trait]
pub trait GoogleIdTokenVerifier: Send + Sync {
    /// Verify the ID token and return trusted claims.
    /// Returns an error if verification fails or required claims are missing.
    async fn verify(&self, id_token: &str) -> Result<VerifiedGoogleClaims, AppError>;
}

/// No-op nonce verifier for ID tokens received outside the authorization flow.
/// We verify signature, issuer, audience, and expiry; nonce is optional for
/// tokens obtained via the frontend OAuth callback.
struct NoOpNonceVerifier;

impl NonceVerifier for NoOpNonceVerifier {
    fn verify(self, _nonce: Option<&openidconnect::Nonce>) -> Result<(), String> {
        Ok(())
    }
}

/// Cached verifier state. Replaced when JWKS is refreshed (key rotation).
struct VerifierState {
    client: CoreClient,
    version: u64,
}

/// Production verifier using OpenID Connect discovery and JWKS.
/// Initialized once at startup; verification is in-memory. On `NoMatchingKey`
/// (key rotation), re-fetches JWKS asynchronously once and retries.
pub struct GoogleVerifierImpl {
    state: Arc<std::sync::RwLock<VerifierState>>,
    refresh_mutex: tokio::sync::Mutex<()>,
    client_id: String,
    issuer_url: IssuerUrl,
    http_client: reqwest::Client,
}

impl GoogleVerifierImpl {
    /// Create verifier by fetching Google's OIDC metadata and JWKS.
    /// Call once at startup; the verifier caches JWKS for subsequent requests.
    pub async fn new(client_id: impl Into<String>) -> Result<Self, AppError> {
        let client_id = client_id.into();
        let issuer_url = IssuerUrl::new("https://accounts.google.com".to_string())
            .map_err(|e| AppError::config_msg(e.to_string(), "Invalid Google issuer URL"))?;

        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| AppError::config_msg(e.to_string(), "Failed to build HTTP client"))?;

        let provider_metadata =
            CoreProviderMetadata::discover_async(issuer_url.clone(), &http_client)
                .await
                .map_err(|e| {
                    AppError::config_msg(e.to_string(), "Failed to fetch Google OIDC discovery")
                })?;

        let client = CoreClient::new(
            ClientId::new(client_id.clone()),
            issuer_url.clone(),
            provider_metadata.jwks().clone(),
        );

        Ok(Self {
            state: Arc::new(std::sync::RwLock::new(VerifierState { client, version: 0 })),
            refresh_mutex: tokio::sync::Mutex::new(()),
            client_id,
            issuer_url,
            http_client,
        })
    }

    /// Fetch provider metadata and JWKS asynchronously (used during refresh).
    async fn fetch_provider_metadata(&self) -> Result<CoreProviderMetadata, AppError> {
        CoreProviderMetadata::discover_async(self.issuer_url.clone(), &self.http_client)
            .await
            .map_err(|e| {
                AppError::config_msg(
                    e.to_string(),
                    "Failed to fetch Google OIDC discovery during JWKS refresh",
                )
            })
    }

    /// Attempt verification with the given client. Returns Ok(claims) or the
    /// raw verification error for inspection.
    fn try_verify(
        client: &CoreClient,
        id_token: &str,
    ) -> Result<VerifiedGoogleClaims, ClaimsVerificationError> {
        let id_token = CoreIdToken::from_str(id_token)
            .map_err(|e| ClaimsVerificationError::Other(e.to_string()))?;

        let verifier = client.id_token_verifier();
        let claims = id_token.claims(&verifier, NoOpNonceVerifier)?;

        let sub = claims.subject().as_str().trim();
        if sub.is_empty() {
            return Err(ClaimsVerificationError::InvalidSubject(
                "missing or empty sub".to_string(),
            ));
        }

        let email = claims.email().map(|e| e.as_str().trim()).unwrap_or("");
        if email.is_empty() {
            return Err(ClaimsVerificationError::Other(
                "missing or empty email".to_string(),
            ));
        }

        if !claims.email_verified().unwrap_or(false) {
            return Err(ClaimsVerificationError::Other(
                "email not verified".to_string(),
            ));
        }

        let name = claims
            .name()
            .and_then(|lc| lc.iter().next().map(|(_, v)| v.as_str().trim()))
            .filter(|s| !s.is_empty())
            .map(String::from);

        Ok(VerifiedGoogleClaims {
            sub: sub.to_string(),
            email: email.to_string(),
            name,
        })
    }

    /// Convert verification error to AppError.
    fn verification_error_to_app_error(e: ClaimsVerificationError) -> AppError {
        tracing::debug!(error = ?e, "Google ID token verification failed");
        AppError::bad_request(
            ErrorCode::InvalidIdToken,
            "Invalid or expired Google ID token".to_string(),
        )
    }

    /// True if the error indicates the signing key may be missing (key rotation).
    fn is_key_rotation_error(e: &ClaimsVerificationError) -> bool {
        matches!(
            e,
            ClaimsVerificationError::SignatureVerification(
                SignatureVerificationError::NoMatchingKey
            )
        )
    }
}

#[async_trait]
impl GoogleIdTokenVerifier for GoogleVerifierImpl {
    async fn verify(&self, id_token: &str) -> Result<VerifiedGoogleClaims, AppError> {
        // 1. Try verification with cached JWKS (normal path, no network).
        let first_result = {
            let state = self.state.read().map_err(|_| {
                AppError::config_msg(
                    "RwLock poisoned".to_string(),
                    "Google verifier state lock poisoned",
                )
            })?;
            Self::try_verify(&state.client, id_token)
        };

        if let Ok(claims) = first_result {
            return Ok(claims);
        }

        let err = match &first_result {
            Ok(_) => unreachable!("returned above on Ok"),
            Err(e) => e.clone(),
        };

        // 2. If not key-rotation related, return immediately.
        if !Self::is_key_rotation_error(&err) {
            return Err(Self::verification_error_to_app_error(err));
        }

        // 3. Acquire refresh lock so only one request performs the refresh.
        let _guard = self.refresh_mutex.lock().await;

        // 4. Check if another request already refreshed while we waited.
        let version_before = {
            let state = self.state.read().map_err(|_| {
                AppError::config_msg(
                    "RwLock poisoned".to_string(),
                    "Google verifier state lock poisoned",
                )
            })?;
            state.version
        };

        let second_result = {
            let state = self.state.read().map_err(|_| {
                AppError::config_msg(
                    "RwLock poisoned".to_string(),
                    "Google verifier state lock poisoned",
                )
            })?;
            Self::try_verify(&state.client, id_token)
        };

        if let Ok(claims) = second_result {
            return Ok(claims);
        }

        // 5. If version changed, another request refreshed; retry once more.
        {
            let state = self.state.read().map_err(|_| {
                AppError::config_msg(
                    "RwLock poisoned".to_string(),
                    "Google verifier state lock poisoned",
                )
            })?;
            if state.version != version_before {
                drop(_guard);
                return Self::try_verify(&state.client, id_token)
                    .map_err(Self::verification_error_to_app_error);
            }
        }

        // 6. We're first; re-fetch provider metadata and JWKS (async, no blocking).
        // Drop any write lock before await so the future is Send.
        let provider_metadata = match self.fetch_provider_metadata().await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "JWKS refresh failed during key rotation");
                drop(_guard);
                return Err(Self::verification_error_to_app_error(err));
            }
        };

        let new_client = CoreClient::new(
            ClientId::new(self.client_id.clone()),
            self.issuer_url.clone(),
            provider_metadata.jwks().clone(),
        );

        {
            let mut state = self.state.write().map_err(|_| {
                AppError::config_msg(
                    "RwLock poisoned".to_string(),
                    "Google verifier state lock poisoned",
                )
            })?;
            state.client = new_client;
            state.version = state.version.wrapping_add(1);
        }
        drop(_guard);

        // 7. Retry verification exactly once with refreshed keys.
        let state = self.state.read().map_err(|_| {
            AppError::config_msg(
                "RwLock poisoned".to_string(),
                "Google verifier state lock poisoned",
            )
        })?;
        Self::try_verify(&state.client, id_token).map_err(Self::verification_error_to_app_error)
    }
}

/// Mock verifier for tests. Returns configured claims for any token.
#[derive(Clone)]
pub struct MockGoogleVerifier {
    pub claims: VerifiedGoogleClaims,
}

impl MockGoogleVerifier {
    pub fn new(claims: VerifiedGoogleClaims) -> Self {
        Self { claims }
    }
}

#[async_trait]
impl GoogleIdTokenVerifier for MockGoogleVerifier {
    async fn verify(&self, _id_token: &str) -> Result<VerifiedGoogleClaims, AppError> {
        Ok(self.claims.clone())
    }
}

/// Type alias for the verifier used in AppState.
pub type GoogleVerifier = Arc<dyn GoogleIdTokenVerifier>;
