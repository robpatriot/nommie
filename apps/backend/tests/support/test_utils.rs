// General test utilities

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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

/// Generate a deterministic seed for a test based on its name.
///
/// Creates a unique, deterministic seed for each test that remains consistent
/// across test runs but differs between tests to avoid data conflicts.
///
/// # Arguments
/// * `test_name` - Name of the test (e.g., "test_full_game_with_ai_players")
///
/// # Returns
/// A seed in the range 10000-1009999
///
/// # Example
/// ```
/// let seed = test_seed("test_full_game_with_ai_players");
/// assert!(seed >= 10000 && seed < 1010000);
/// ```
pub fn test_seed(test_name: &str) -> i64 {
    let mut hasher = DefaultHasher::new();
    test_name.hash(&mut hasher);
    (hasher.finish() % 1_000_000) as i64 + 10000 // Range: 10000-1009999
}

/// Generate a deterministic seed as u64 for functions that require u64 seeds.
///
/// This is a convenience function for cases where u64 is required instead of i64.
///
/// # Arguments
/// * `test_name` - Name of the test (e.g., "test_full_game_with_ai_players")
///
/// # Returns
/// A seed in the range 10000-1009999 as u64
pub fn test_seed_u64(test_name: &str) -> u64 {
    test_seed(test_name) as u64
}

/// Generate a unique user sub for a test based on its name.
///
/// Creates a consistent, test-specific user sub that avoids conflicts
/// between concurrent tests while remaining deterministic.
///
/// # Arguments
/// * `test_name` - Name of the test (e.g., "test_full_game_with_ai_players")
///
/// # Returns
/// A user sub string in format "test_{sanitized_test_name}"
///
/// # Example
/// ```
/// let sub = test_user_sub("test_full_game_with_ai_players");
/// assert_eq!(sub, "test_test_full_game_with_ai_players");
/// ```
pub fn test_user_sub(test_name: &str) -> String {
    format!("test_{}", test_name.replace("::", "_"))
}
