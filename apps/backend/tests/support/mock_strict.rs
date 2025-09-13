use std::collections::HashSet;
use std::sync::{Mutex, Once, OnceLock};

use sea_orm::DatabaseConnection;

static REGISTRY: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();
static DETECTOR_INSTALLED: Once = Once::new();

fn registry() -> &'static Mutex<HashSet<usize>> {
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

fn key(conn: &DatabaseConnection) -> usize {
    (conn as *const DatabaseConnection) as usize
}

/// Check if the given database connection is registered as a mock strict connection.
/// This reads the local registry directly (no call to the backend hook).
pub fn is_registered_mock_strict(conn: &DatabaseConnection) -> bool {
    let set = registry().lock().expect("mock registry poisoned");
    set.contains(&key(conn))
}

pub fn register_mock_strict_connection(conn: &DatabaseConnection) {
    // Install the detector function exactly once
    // We pass the local registry fn, not the backend hook, to avoid recursion
    DETECTOR_INSTALLED.call_once(|| {
        backend::infra::mock_strict::set_mock_strict_detector(is_registered_mock_strict);
    });

    // Insert the connection key into the registry
    let mut set = registry().lock().expect("mock registry poisoned");
    set.insert(key(conn));
}
