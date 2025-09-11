#![cfg(test)]

//! Unified test logging initialization
//!
//! This module provides a single source of truth for test logging initialization
//! that works for both unit tests and integration tests. It uses a one-time guard
//! to prevent double initialization and integrates cleanly with cargo/nextest output capture.

use once_cell::sync::OnceCell;
use tracing_subscriber::{fmt, EnvFilter};

static INITIALIZED: OnceCell<()> = OnceCell::new();

/// Initialize structured logging for tests.
///
/// This function is idempotent and race-safe. It can be called multiple times
/// without panicking. The logging level is controlled in this order of precedence:
///
/// 1. `TEST_LOG` environment variable (preferred)
/// 2. `RUST_LOG` environment variable (fallback)
/// 3. `"warn"` (default, quiet)
///
/// The subscriber is configured with:
/// - `with_test_writer()` for cargo/nextest output capture
/// - `without_time()` for stable, clean output
/// - `try_init().ok()` to never panic if already initialized
///
/// # Example Usage
///
/// ```rust
/// use backend::test_bootstrap::logging::init;
///
/// #[tokio::test]
/// async fn test_with_logs() {
///     init(); // Safe to call multiple times
///     tracing::info!("This will be visible when TEST_LOG=info");
/// }
/// ```
///
/// # Environment Variables
///
/// ```bash
/// # Use TEST_LOG (preferred)
/// TEST_LOG=info pnpm be:test:v
///
/// # Fallback to RUST_LOG
/// RUST_LOG=debug pnpm be:test:v
///
/// # Default (warn level)
/// pnpm be:test
/// ```
pub fn init() {
    INITIALIZED.get_or_init(|| {
        // Read log level in order: TEST_LOG -> RUST_LOG -> "warn"
        let filter = std::env::var("TEST_LOG")
            .or_else(|_| std::env::var("RUST_LOG"))
            .map(EnvFilter::new)
            .unwrap_or_else(|_| EnvFilter::new("warn"));

        fmt()
            .with_env_filter(filter)
            .with_test_writer() // Critical for cargo/nextest capture
            .without_time() // Stable output
            .try_init()
            .ok(); // Never panic if something else already initialized
    });
}
