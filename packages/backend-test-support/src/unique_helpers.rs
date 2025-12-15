//! Test helpers for generating unique test data
//!
//! This module provides utilities to help generate unique test data using ULIDs
//! to ensure test isolation and avoid conflicts between test runs.

use ulid::Ulid;

/// Generate a unique string with the given prefix
///
/// # Arguments
/// * `prefix` - The prefix to use for the unique string
///
/// # Returns
/// A unique string in the format `{prefix}-{ulid}`
///
/// # Examples
/// ```
/// use backend_test_support::unique_helpers::{unique_str, unique_email};
///
/// let id1 = unique_str("user");
/// let id2 = unique_str("user");
/// assert_ne!(id1, id2);
/// assert!(id1.starts_with("user-"));
/// ```
pub fn unique_str(prefix: &str) -> String {
    format!("{}-{}", prefix, Ulid::new())
}

/// Generate a unique email address with the given prefix
///
/// # Arguments
/// * `prefix` - The prefix to use for the email address
///
/// # Returns
/// A unique email address in the format `{prefix}-{ulid}@example.test`
///
/// # Examples
/// ```
/// use backend_test_support::unique_helpers::unique_email;
///
/// let email1 = unique_email("test");
/// let email2 = unique_email("test");
/// assert_ne!(email1, email2);
/// assert!(email1.ends_with("@example.test"));
/// assert!(email1.starts_with("test-"));
/// ```
pub fn unique_email(prefix: &str) -> String {
    format!("{}-{}@example.test", prefix, Ulid::new())
}

