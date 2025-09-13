mod support;

use backend::config::db::{DbOwner, DbProfile};
use backend::infra::db::connect_db;
use backend::infra::mock_strict::is_mock_strict;
use sea_orm::{DatabaseBackend, MockDatabase};
use support::mock_strict::{is_registered_mock_strict, register_mock_strict_connection};

#[tokio::test]
async fn test_mock_strict_detector_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Test 1: Build a strict mock connection and verify it's detected
    let mock_db = MockDatabase::new(DatabaseBackend::Postgres);
    let mock_conn = mock_db.into_connection();

    // Initially, the connection should not be detected as mock strict in either place
    assert!(!is_registered_mock_strict(&mock_conn));
    assert!(!is_mock_strict(&mock_conn));

    // Register the mock connection
    register_mock_strict_connection(&mock_conn);

    // Now both should detect it as mock strict
    assert!(is_registered_mock_strict(&mock_conn));
    assert!(is_mock_strict(&mock_conn));

    // Test 2: Build a real Test DB connection and verify it's not detected as mock strict
    let real_conn = connect_db(DbProfile::Test, DbOwner::App).await?;
    assert!(!is_registered_mock_strict(&real_conn));
    assert!(!is_mock_strict(&real_conn));

    Ok(())
}
