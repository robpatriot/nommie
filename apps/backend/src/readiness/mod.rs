pub mod manager;
pub mod monitor;
pub mod types;

/// Suggested Retry-After (seconds) for 503 responses when the service is not ready.
/// Aligns with startup poll interval; used by ReadinessGate and health routes.
pub const READINESS_RETRY_AFTER_SECS: u32 = 1;

pub use manager::ReadinessManager;
pub use types::{CheckStatus, DependencyName, DependencyStatus, MigrationState, ServiceMode};
