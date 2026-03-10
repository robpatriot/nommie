//! Admission mode for first-time signup.
//!
//! Admission policy is deployment config, sourced from environment at startup.
//! ALLOWED_EMAILS unset or empty → open; ALLOWED_EMAILS set and non-empty → restricted.

use std::env;

/// Admission mode for first-time login.
///
/// - **Open**: All emails are admitted for first-time signup. No admission table check.
/// - **Restricted**: Email must match at least one pattern in the admission table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionMode {
    /// All emails admitted; no admission table check for allow/deny.
    Open,
    /// Email must match admission rules in the database.
    Restricted,
}

impl AdmissionMode {
    /// Parse admission mode from environment.
    /// ALLOWED_EMAILS unset or empty → Open; otherwise → Restricted.
    pub fn from_env() -> Self {
        match env::var("ALLOWED_EMAILS") {
            Ok(v) if !v.trim().is_empty() => Self::Restricted,
            _ => Self::Open,
        }
    }
}
