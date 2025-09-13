use std::sync::OnceLock;

use sea_orm::DatabaseConnection;

static DETECTOR: OnceLock<fn(&DatabaseConnection) -> bool> = OnceLock::new();

/// Set the mock strict detector function.
/// This can only be called once per program execution.
pub fn set_mock_strict_detector(detector: fn(&DatabaseConnection) -> bool) {
    let _ = DETECTOR.set(detector);
}

/// Check if the given database connection is a mock strict connection.
/// Returns false if no detector has been set.
pub fn is_mock_strict(conn: &DatabaseConnection) -> bool {
    if let Some(detector) = DETECTOR.get() {
        detector(conn)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{DatabaseBackend, MockDatabase};

    use super::*;

    #[test]
    fn test_default_behavior_no_detector() {
        // Create a real mock connection for testing
        let mock_db = MockDatabase::new(DatabaseBackend::Postgres);
        let mock_conn = mock_db.into_connection();

        // Should return false when no detector is installed
        assert!(!is_mock_strict(&mock_conn));
    }
}
