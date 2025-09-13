mod support;

use backend::config::db::DbProfile;
use backend::infra::mock_strict::is_mock_strict;
use backend::infra::state::StateBuilder;
use backend::state::security_config::SecurityConfig;
use sea_orm::{DatabaseBackend, MockDatabase};
use support::mock_strict::{is_registered_mock_strict, register_mock_strict_connection};

#[tokio::test]
async fn test_mock_strict_hook_wiring() -> Result<(), Box<dyn std::error::Error>> {
    // Test 1: Build a strict mock connection and verify both local registry and hook work
    let mock_db = MockDatabase::new(DatabaseBackend::Postgres);
    let mock_conn = mock_db.into_connection();

    // Initially, the connection should not be detected as mock strict in either place
    assert!(!is_registered_mock_strict(&mock_conn));
    assert!(!is_mock_strict(&mock_conn));

    // Register the mock connection
    register_mock_strict_connection(&mock_conn);

    // Now both should detect it as mock strict
    assert!(
        is_registered_mock_strict(&mock_conn),
        "Local registry should detect mock strict connection"
    );
    assert!(
        is_mock_strict(&mock_conn),
        "Backend hook should mirror the registry"
    );

    // Test 2: Build a real Test DB connection and verify both return false
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = StateBuilder::new()
        .with_db(DbProfile::Test)
        .with_security(security_config)
        .build()
        .await?;

    let real_conn = &state.db;

    // Real connections should not be detected as mock strict in either place
    assert!(
        !is_registered_mock_strict(real_conn),
        "Local registry should not detect real connection as mock strict"
    );
    assert!(
        !is_mock_strict(real_conn),
        "Backend hook should not detect real connection as mock strict"
    );

    Ok(())
}
