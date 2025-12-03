// SQLite backend performance tests
//
// Tests comparing SQLite in-memory vs file-based database performance.

use std::time::Instant;

use backend::config::db::{DbKind, RuntimeEnv};
use backend::db::txn::with_txn;
use backend::infra::state::build_state;
use backend::services::users::UserService;
use backend::utils::unique::{unique_email, unique_str};
use tracing::info;

#[tokio::test]
#[ignore]
async fn memory_vs_file_performance_sqlite() -> Result<(), Box<dyn std::error::Error>> {
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
                    .ensure_user(txn, &email, Some(&format!("Memory User {}", i)), &sub, None)
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

    let start = Instant::now();
    with_txn(None, &file_state, |txn| {
        Box::pin(async move {
            let service = UserService;
            for i in 0..10 {
                let email = unique_email(&format!("file_user_{}", i));
                let sub = unique_str(&format!("file-sub-{}", i));
                let _user = service
                    .ensure_user(txn, &email, Some(&format!("File User {}", i)), &sub, None)
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
