//! MockStrict registry for test database connections
//!
//! This module provides a registry to track which database connections
//! are MockStrict test connections, allowing tests to verify they're
//! using the correct database type.

use sea_orm::DatabaseConnection;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

static REGISTRY: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashSet<usize>> {
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

fn key(conn: &DatabaseConnection) -> usize {
    (conn as *const DatabaseConnection) as usize
}

/// Mark a connection as a MockStrict test connection.
///
/// This should be called when setting up MockStrict database connections
/// in tests to register them in the global registry.
///
/// # Arguments
/// * `conn` - The database connection to register
pub fn register_mock_strict_connection(conn: &DatabaseConnection) {
    let mut set = registry().lock().expect("mock registry poisoned");
    set.insert(key(conn));
}

/// Returns true if this connection was registered as MockStrict.
///
/// This can be used in tests to verify that the correct database
/// connection type is being used.
///
/// # Arguments
/// * `conn` - The database connection to check
///
/// # Returns
/// `true` if the connection was registered as MockStrict, `false` otherwise
pub fn is_mock_strict(conn: &DatabaseConnection) -> bool {
    let set = registry().lock().expect("mock registry poisoned");
    set.contains(&key(conn))
}
