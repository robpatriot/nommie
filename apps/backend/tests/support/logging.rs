#![cfg(test)]

//! Test logging helper for debug output
//!
//! This module provides a simple way to enable structured logging in tests that need debug output.
//! It's designed to be opt-in only - tests remain quiet by default. Only use this when a test
//! needs to output debug information, and call it directly from that test.
//!
//! # Example Usage
//!
//! ```rust
//! use backend::tests::support::logging;
//!
//! #[tokio::test]
//! async fn test_with_logs() {
//!     // Enable logging for this test
//!     logging::init();
//!     
//!     // Your test code here - logs will be captured and shown
//!     tracing::info!("This will be visible when TEST_LOG=info");
//! }
//! ```
//!
//! # Example Commands
//!
//! ```bash
//! # Run with info-level logs
//! TEST_LOG=info pnpm be:test:v -- some_filter
//!
//! # Run with debug-level logs
//! TEST_LOG=debug pnpm be:test:v -- some_filter
//!
//! # Run with trace-level logs
//! TEST_LOG=trace pnpm be:test:v -- some_filter
//! ```

use std::sync::OnceLock;

use tracing_subscriber::{fmt, EnvFilter};

static INITIALIZED: OnceLock<()> = OnceLock::new();

/// Initialize structured logging for tests.
///
/// This function is idempotent and race-safe. It can be called multiple times
/// without panicking. The logging level is controlled by the `TEST_LOG` environment
/// variable, which accepts:
///
/// - Simple levels: `info`, `debug`, `trace`, `warn`, `error`
/// - Full EnvFilter strings: `backend=debug,actix_web=warn`
///
/// If `TEST_LOG` is not set, defaults to `warn` level.
#[allow(dead_code)]
pub fn init() {
    INITIALIZED.get_or_init(|| {
        let filter = std::env::var("TEST_LOG")
            .map(EnvFilter::new)
            .unwrap_or_else(|_| EnvFilter::new("warn"));

        fmt()
            .with_env_filter(filter)
            .with_test_writer() // Integrates with test output capture
            .without_time() // Clean, stable output
            .init();
    });
}
