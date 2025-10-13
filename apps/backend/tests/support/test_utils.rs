//! General test utilities

use ulid::Ulid;

/// Generate a short join code for test games.
///
/// Creates a 10-character join code from a ULID for test purposes.
/// Each call generates a unique code.
///
/// # Example
/// ```
/// let code = short_join_code();
/// assert_eq!(code.len(), 10);
/// ```
pub fn short_join_code() -> String {
    format!("{}", Ulid::new()).chars().take(10).collect()
}
