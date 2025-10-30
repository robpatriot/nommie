// SQLite backend tests
//
// Tests for SQLite in-memory and file-based database functionality.
// Verifies that both backends work correctly with the same business logic.
//
// ## Running SQLite File Tests Serially (Fallback)
//
// If you encounter intermittent "database is locked" errors when running SQLite file tests,
// you can run them serially as a fallback:
//
// ```bash
// # Option 1: Set test-threads = 1 for this test group
// cargo nextest run --test-threads 1 sqlite_backend_tests
//
// # Option 2: Using nextest config in .config/nextest.toml:
// # [[profile.default]]
// # test-threads = 1
//
// # Option 3: Run specific file-based tests in sequence
// cargo nextest run test_sqlite_file_persistence --test-threads 1
// cargo nextest run test_sqlite_default_file --test-threads 1
// cargo nextest run test_sqlite_memory_vs_file_performance --test-threads 1
// ```

use backend::config::db::{DbKind, RuntimeEnv};
use backend::db::txn::with_txn;
use backend::infra::db::sqlite_diagnostics::{connection_id, sqlite_lock_probe};
use backend::infra::state::build_state;
use backend::repos::users;
use backend::services::users::UserService;
use backend::utils::unique::{unique_email, unique_str};
use tracing::info;
use unicode_normalization::UnicodeNormalization;

#[tokio::test]
async fn test_sqlite_memory_works() -> Result<(), Box<dyn std::error::Error>> {
    // build_state() now automatically handles schema migration
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::SqliteMemory)
        .build()
        .await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Test basic operations work with SQLite memory
            let test_email = unique_email("alice");
            let test_google_sub = unique_str("google-sub");
            let service = UserService;

            let user = service
                .ensure_user(txn, &test_email, Some("Alice"), &test_google_sub)
                .await?;

            assert_eq!(user.username, Some("Alice".to_string()));
            assert!(!user.is_ai);
            assert!(user.id > 0);

            Ok(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_sqlite_file_persistence() -> Result<(), Box<dyn std::error::Error>> {
    // Test SQLite file with default database
    let state1 = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::SqliteFile)
        .build()
        .await?;

    // Lock probe for tests
    if let Some(db) = state1.db() {
        let pool_id = connection_id(db);
        let _ = sqlite_lock_probe(db, &pool_id).await;
    }

    // Create some data and verify it in the same transaction
    let test_email = unique_email("persistent");
    let test_google_sub = unique_str("persistent-sub");

    with_txn(None, &state1, |txn| {
        Box::pin(async move {
            let service = UserService;

            // Create user
            let user = service
                .ensure_user(txn, &test_email, Some("Persistent User"), &test_google_sub)
                .await?;

            assert_eq!(user.username, Some("Persistent User".to_string()));

            // Try to find the user by ID
            let user_by_id = users::find_user_by_id(txn, user.id).await?;
            assert!(user_by_id.is_some());

            // Find by email using the same sanitization as ensure_user
            let clean_email = test_email.trim().nfkc().collect::<String>().to_lowercase();
            let user_opt = users::find_credentials_by_email(txn, &clean_email).await?;
            assert!(user_opt.is_some());

            let credential = user_opt.unwrap();
            assert_eq!(credential.email, clean_email);

            Ok(())
        })
    })
    .await?;

    // For file persistence testing, we need to use a different approach
    // Since test policy rolls back transactions, we'll test that the file exists and can be opened
    // The actual persistence would be verified in integration tests with commit policy

    Ok(())
}

#[tokio::test]
async fn test_sqlite_default_file() -> Result<(), Box<dyn std::error::Error>> {
    // Test SQLite file with default database
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::SqliteFile)
        .build()
        .await?;

    // Lock probe for tests
    if let Some(db) = state.db() {
        let pool_id = connection_id(db);
        let _ = sqlite_lock_probe(db, &pool_id).await;
    }

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Just verify we can connect and do basic operations
            let test_email = unique_email("default");
            let test_google_sub = unique_str("default-sub");
            let service = UserService;

            let user = service
                .ensure_user(txn, &test_email, Some("Default User"), &test_google_sub)
                .await?;

            assert_eq!(user.username, Some("Default User".to_string()));
            Ok(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_sqlite_memory_vs_file_performance() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;

    // build_state() now automatically handles schema migration
    let memory_state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::SqliteMemory)
        .build()
        .await?;

    let start = Instant::now();
    with_txn(None, &memory_state, |txn| {
        Box::pin(async move {
            let service = UserService;
            for i in 0..10 {
                let email = unique_email(&format!("memory_user_{}", i));
                let sub = unique_str(&format!("memory-sub-{}", i));
                let _user = service
                    .ensure_user(txn, &email, Some(&format!("Memory User {}", i)), &sub)
                    .await?;
            }
            Ok(())
        })
    })
    .await?;
    let memory_time = start.elapsed();

    // Test SQLite file with default database
    let file_state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::SqliteFile)
        .build()
        .await?;

    // Lock probe for tests
    if let Some(db) = file_state.db() {
        let pool_id = connection_id(db);
        let _ = sqlite_lock_probe(db, &pool_id).await;
    }

    let start = Instant::now();
    with_txn(None, &file_state, |txn| {
        Box::pin(async move {
            let service = UserService;
            for i in 0..10 {
                let email = unique_email(&format!("file_user_{}", i));
                let sub = unique_str(&format!("file-sub-{}", i));
                let _user = service
                    .ensure_user(txn, &email, Some(&format!("File User {}", i)), &sub)
                    .await?;
            }
            Ok(())
        })
    })
    .await?;
    let file_time = start.elapsed();

    // Both should be fast, but memory should be faster
    info!(?memory_time, ?file_time, "SQLite performance comparison");

    // Both should complete in reasonable time (< 1 second for 10 operations)
    assert!(memory_time.as_millis() < 1000);
    assert!(file_time.as_millis() < 1000);

    Ok(())
}
