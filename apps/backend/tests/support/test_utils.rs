// General test utilities

/// Generate a deterministic 32-byte seed for a test based on its name.
///
/// Creates a unique, deterministic seed for each test that remains consistent
/// across test runs but differs between tests to avoid data conflicts.
///
/// # Arguments
/// * `test_name` - Name of the test (e.g., "test_full_game_with_ai_players")
///
/// # Returns
/// A 32-byte seed derived from the test name hash
///
/// # Example
/// ```
/// let seed = test_seed("test_full_game_with_ai_players");
/// assert_eq!(seed.len(), 32);
/// ```
pub fn test_seed(test_name: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(test_name.as_bytes());
    *hasher.finalize().as_bytes()
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
