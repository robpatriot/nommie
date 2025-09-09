//! Test support utilities for generating unique test data
//!
//! This crate provides utilities to help generate unique test data using ULIDs
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
/// use test_support::unique_str;
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
/// use test_support::unique_email;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_str_produces_different_results() {
        let str1 = unique_str("test");
        let str2 = unique_str("test");
        assert_ne!(str1, str2);
    }

    #[test]
    fn test_unique_str_has_correct_prefix() {
        let result = unique_str("user");
        assert!(result.starts_with("user-"));
    }

    #[test]
    fn test_unique_email_produces_different_results() {
        let email1 = unique_email("test");
        let email2 = unique_email("test");
        assert_ne!(email1, email2);
    }

    #[test]
    fn test_unique_email_ends_with_example_test() {
        let email = unique_email("user");
        assert!(email.ends_with("@example.test"));
    }

    #[test]
    fn test_unique_email_has_correct_prefix() {
        let email = unique_email("test");
        assert!(email.starts_with("test-"));
    }

    #[test]
    fn test_unique_email_has_correct_format() {
        let email = unique_email("user");
        // Should be: user-{ulid}@example.test
        let parts: Vec<&str> = email.split('@').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1], "example.test");
        assert!(parts[0].starts_with("user-"));
        assert!(parts[0].len() > 5); // user- + at least some ULID characters
    }
}
