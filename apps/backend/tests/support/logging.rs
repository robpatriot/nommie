//! Unified test logging initialization for integration tests
//!
//! This module provides the same logging initialization logic as the crate's
//! test_bootstrap module, but implemented directly for integration tests since
//! they can't access the crate's test-only modules.
//!
//! # Example Usage
//!
//! ```rust
//! use backend::tests::support::logging;
//!
//! #[tokio::test]
//! async fn test_with_logs() {
//!     logging::init(); // Uses the unified initializer
//!     tracing::info!("This will be visible when TEST_LOG=info");
//! }
//! ```
//!
//! # Environment Variables
//!
//! The unified initializer respects these in order of precedence:
//! 1. `TEST_LOG` (preferred)
//! 2. `RUST_LOG` (fallback)
//! 3. `"warn"` (default, quiet)
//!
//! ```bash
//! # Use TEST_LOG (preferred)
//! TEST_LOG=info pnpm be:test:v
//!
//! # Fallback to RUST_LOG
//! RUST_LOG=debug pnpm be:test:v
//!
//! # Default (warn level)
//! pnpm be:test
//! ```

use once_cell::sync::OnceCell;
use tracing_subscriber::{fmt, EnvFilter};

static INITIALIZED: OnceCell<()> = OnceCell::new();

/// Initialize structured logging for integration tests.
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

/// Automatically initialize logging for all integration test binaries.
///
/// This constructor runs once per integration test binary, ensuring logging
/// is set up before any tests run. The OnceCell guard prevents double initialization.
#[ctor::ctor]
fn _auto_init_for_integration_tests() {
    init();
}
