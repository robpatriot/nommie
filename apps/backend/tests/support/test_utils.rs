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

/// Get a shared temporary directory for SQLite file-based tests within the same nextest run.
///
/// Creates a stable directory path that is shared across all parallel workers in the
/// same nextest run, but unique across different runs. This allows file-based
/// SQLite tests to test cross-process database contention and use different
/// database files within the same directory.
///
/// The directory is created within the system temp directory to keep test files organized.
///
/// # Returns
/// A PathBuf pointing to the shared temporary directory for SQLite databases
///
/// # Examples
/// ```
/// let temp_dir = shared_sqlite_temp_dir();
/// let db_path = temp_dir.join("test.db");
/// // db_path is ready to use directly with SQLite
/// ```
pub fn shared_sqlite_temp_dir() -> std::path::PathBuf {
    // Create a dedicated subdirectory for nommie test files
    let test_dir = std::env::temp_dir().join("nommie-tests");

    // Ensure the test directory exists
    if let Err(e) = std::fs::create_dir_all(&test_dir) {
        panic!("Failed to create test directory {:?}: {}", test_dir, e);
    }

    test_dir
}

/// Get the shared SQLite database file path for all integration tests.
///
/// All SQLite file-based tests should use this function to ensure they use
/// the same database file. This allows tests to share state and verify
/// cross-test persistence scenarios.
///
/// The file is created within the shared temporary directory to keep test files organized.
///
/// # Returns
/// A PathBuf pointing to the shared SQLite database file (test.db)
///
/// # Examples
/// ```
/// let db_path = shared_sqlite_temp_file();
/// // db_path points to the shared database file used by all tests
/// ```
pub fn shared_sqlite_temp_file() -> std::path::PathBuf {
    shared_sqlite_temp_dir().join("test.db")
}
