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

/// Automatically initialize logging for all integration test binaries.
///
/// This constructor runs once per integration test binary, ensuring logging
/// is set up before any tests run using the unified logging initialization.
#[ctor::ctor]
fn _auto_init_for_integration_tests() {
    backend_test_support::test_logging::init();
}
