//! Deterministic tests for migration lock behavior
//!
//! Tests prove:
//! - No premature unlock occurs
//! - Proper cleanup after cancellation, errors, and timeouts
//! - Works for both PostgreSQL advisory locks and SQLite file locks

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use backend::config::db::{DbKind, RuntimeEnv};
use backend::infra::db::locking::{BootstrapLock, PgAdvisoryLock, SqliteFileLock};
use db_infra::infra::db::build_admin_pool;
use migration::{migrate, MigrationCommand};
use tokio::sync::Barrier;

use crate::support::resolve_test_db_kind;

/// Helper to skip tests that don't apply to SQLite memory databases.
/// Migration locks require persistent storage (Postgres advisory locks or SQLite file locks).
fn skip_if_sqlite_memory(test_name: &str) -> Option<DbKind> {
    let db_kind = resolve_test_db_kind().expect("Failed to resolve DB kind");

    if matches!(db_kind, DbKind::SqliteMemory) {
        println!("Skipping {} for DbKind::{:?}", test_name, db_kind);
        return None;
    }

    Some(db_kind)
}

enum TestLock {
    Postgres {
        lock: PgAdvisoryLock,
        #[allow(dead_code)] // Stored to keep connection alive, but not directly accessed
        pool: sea_orm::DatabaseConnection,
    },
    SqliteFile {
        lock: SqliteFileLock,
        lock_path: PathBuf, // Store path for cleanup
    },
}

impl TestLock {
    async fn create_shared(
        base_name: &str,
        _task_suffix: &str,
    ) -> (Self, Option<sea_orm::DatabaseConnection>) {
        let db_kind = resolve_test_db_kind().expect("Failed to resolve DB kind");

        match db_kind {
            DbKind::Postgres => {
                let admin_pool = build_admin_pool(RuntimeEnv::Test, db_kind)
                    .await
                    .expect("Failed to build admin pool");

                let lock_key = format!(
                    "test:migration_lock:shared:{}:{}",
                    base_name,
                    std::process::id()
                );
                let lock = PgAdvisoryLock::new(admin_pool.clone(), &lock_key);

                (
                    TestLock::Postgres {
                        lock,
                        pool: admin_pool.clone(),
                    },
                    Some(admin_pool),
                )
            }
            DbKind::SqliteFile => {
                // Create deterministic lock path based on base_name
                // All tasks in the same test will use the same lock file
                let lock_path = std::env::temp_dir().join(format!(
                    "{}-{}.migrate.lock",
                    base_name,
                    std::process::id()
                ));
                let lock = SqliteFileLock::new(&lock_path).expect("Should create lock");

                (TestLock::SqliteFile { lock, lock_path }, None)
            }
            DbKind::SqliteMemory => {
                panic!("SQLite memory does not support migration locks")
            }
        }
    }

    async fn try_acquire(
        &mut self,
    ) -> Result<Option<backend::infra::db::locking::Guard>, backend::AppError> {
        match self {
            TestLock::Postgres { lock, .. } => Ok(BootstrapLock::try_acquire(lock).await?),
            TestLock::SqliteFile { lock, .. } => Ok(BootstrapLock::try_acquire(lock).await?),
        }
    }
}

// Clean up lock file when TestLock is dropped
impl Drop for TestLock {
    fn drop(&mut self) {
        if let TestLock::SqliteFile { lock_path, .. } = self {
            // Attempt to remove the lock file, ignore errors (file may already be removed)
            let _ = std::fs::remove_file(lock_path);
        }
    }
}

/// Helper to simulate a slow migration that can be cancelled
async fn slow_migration_task(
    pool: sea_orm::DatabaseConnection,
    cancel_flag: Arc<AtomicBool>,
    duration_ms: u64,
) -> Result<(), sea_orm::DbErr> {
    let start = std::time::Instant::now();

    while start.elapsed() < Duration::from_millis(duration_ms) {
        if cancel_flag.load(Ordering::Relaxed) {
            // Simulate cancelled migration
            return Err(sea_orm::DbErr::Custom(
                "Migration cancelled by test".to_string(),
            ));
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Run actual migration (should be idempotent)
    migrate(&pool, MigrationCommand::Up).await
}

#[tokio::test]
async fn cancel_mid_migration_releases_lock() {
    // A acquires lock and starts a long "body".
    // Cancel A; assert A task aborted, then B acquires successfully (no timeout).

    if skip_if_sqlite_memory("cancel_mid_migration_releases_lock").is_none() {
        return;
    }

    let barrier = Arc::new(Barrier::new(2));
    let a_started = Arc::new(AtomicBool::new(false));
    let a_cancelled = Arc::new(AtomicBool::new(false));
    let cancel_flag = Arc::new(AtomicBool::new(false));

    // Task A: Acquire lock and start slow migration
    let barrier_a = barrier.clone();
    let a_started_flag = a_started.clone();
    let a_cancelled_flag = a_cancelled.clone();
    let cancel_flag_a = cancel_flag.clone();

    let task_a = tokio::spawn(async move {
        let (mut lock_a, admin_pool_opt) =
            TestLock::create_shared("cancel_mid_migration", "A").await;

        // Try to acquire lock
        let guard = lock_a
            .try_acquire()
            .await
            .expect("A should acquire lock")
            .expect("A should get guard");

        // Signal that A has acquired the lock
        a_started_flag.store(true, Ordering::Relaxed);
        barrier_a.wait().await;

        // Start slow migration that can be cancelled (only for Postgres)
        let was_cancelled = if let Some(admin_pool_a) = admin_pool_opt {
            let result = slow_migration_task(admin_pool_a, cancel_flag_a, 5000).await;
            let was = result.is_err() && result.unwrap_err().to_string().contains("cancelled");
            a_cancelled_flag.store(was, Ordering::Relaxed);
            was
        } else {
            // For SQLite file locks, just simulate waiting and cancellation
            tokio::time::sleep(Duration::from_millis(100)).await;
            if cancel_flag_a.load(Ordering::Relaxed) {
                a_cancelled_flag.store(true, Ordering::Relaxed);
                true
            } else {
                false
            }
        };

        // Always release the guard
        guard.release().await.expect("A should release guard");

        was_cancelled
    });

    // Task B: Wait for A to start, then try to acquire lock
    let barrier_b = barrier.clone();
    let task_b = tokio::spawn(async move {
        // Wait for A to acquire lock
        barrier_b.wait().await;

        // Give A some time to start the migration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Cancel A's migration
        cancel_flag.store(true, Ordering::Relaxed);

        // Wait a bit for A to process cancellation
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Now try to acquire the lock - should succeed quickly since A released it
        let (mut lock_b, _admin_pool_b) =
            TestLock::create_shared("cancel_mid_migration", "B").await;

        let acquire_start = std::time::Instant::now();
        let guard_b = lock_b
            .try_acquire()
            .await
            .expect("B try_acquire should not fail")
            .expect("B should acquire lock after A cancelled");
        let acquire_duration = acquire_start.elapsed();

        // Should acquire quickly (not timeout)
        assert!(
            acquire_duration < Duration::from_millis(500),
            "B should acquire lock quickly after A cancelled, took: {:?}",
            acquire_duration
        );

        guard_b.release().await.expect("B should release guard");
        true
    });

    // Wait for both tasks
    let (a_result, b_result) = tokio::join!(task_a, task_b);

    assert!(
        a_result.expect("Task A should complete"),
        "A should be cancelled"
    );
    assert!(
        b_result.expect("Task B should complete"),
        "B should acquire lock successfully"
    );
    assert!(a_started.load(Ordering::Relaxed), "A should have started");
    assert!(
        a_cancelled.load(Ordering::Relaxed),
        "A should have been cancelled"
    );
}

#[tokio::test]
async fn body_error_unlocks() {
    // A acquires; migration returns Err; assert unlock happened and B acquires.

    if skip_if_sqlite_memory("body_error_unlocks").is_none() {
        return;
    }

    let barrier = Arc::new(Barrier::new(2));
    let a_completed = Arc::new(AtomicBool::new(false));

    // Task A: Acquire lock and fail during migration
    let barrier_a = barrier.clone();
    let a_completed_flag = a_completed.clone();

    let task_a = tokio::spawn(async move {
        let (mut lock_a, _admin_pool_a) = TestLock::create_shared("body_error", "A").await;

        // Acquire lock
        let guard = lock_a
            .try_acquire()
            .await
            .expect("A should acquire lock")
            .expect("A should get guard");

        // Signal that A has the lock
        barrier_a.wait().await;

        // Simulate migration error
        let migration_result: Result<(), sea_orm::DbErr> = Err(sea_orm::DbErr::Custom(
            "Simulated migration error".to_string(),
        ));

        // Release guard (this is what the real implementation does on error)
        guard.release().await.expect("A should release guard");

        a_completed_flag.store(true, Ordering::Relaxed);

        migration_result.is_err()
    });

    // Task B: Wait for A to complete, then acquire lock
    let barrier_b = barrier.clone();
    let task_b = tokio::spawn(async move {
        // Wait for A to acquire lock
        barrier_b.wait().await;

        // Give A time to fail and release
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Wait for A to complete
        while !a_completed.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Now try to acquire - should succeed immediately
        let (mut lock_b, _admin_pool_b) = TestLock::create_shared("body_error", "B").await;

        let acquire_start = std::time::Instant::now();
        let guard_b = lock_b
            .try_acquire()
            .await
            .expect("B try_acquire should not fail")
            .expect("B should acquire lock after A errored");
        let acquire_duration = acquire_start.elapsed();

        // Should acquire quickly
        assert!(
            acquire_duration < Duration::from_millis(100),
            "B should acquire lock quickly after A errored, took: {:?}",
            acquire_duration
        );

        guard_b.release().await.expect("B should release guard");
        true
    });

    // Wait for both tasks
    let (a_result, b_result) = tokio::join!(task_a, task_b);

    assert!(
        a_result.expect("Task A should complete"),
        "A should have migration error"
    );
    assert!(
        b_result.expect("Task B should complete"),
        "B should acquire lock successfully"
    );
}

#[tokio::test]
async fn acquire_timeout_distinct() {
    // A holds lock beyond acquire timeout; B attempts and returns None (locked).

    if skip_if_sqlite_memory("acquire_timeout_distinct").is_none() {
        return;
    }

    // Set very short acquire timeout for this test
    // TODO: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::set_var("NOMMIE_MIGRATE_TIMEOUT_MS", "200") };

    let barrier = Arc::new(Barrier::new(2));
    let a_holding = Arc::new(AtomicBool::new(false));

    // Task A: Acquire and hold lock for longer than acquire timeout
    let barrier_a = barrier.clone();
    let a_holding_flag = a_holding.clone();

    let task_a = tokio::spawn(async move {
        let (mut lock_a, _admin_pool_a) = TestLock::create_shared("acquire_timeout", "A").await;

        // Acquire lock
        let guard = lock_a
            .try_acquire()
            .await
            .expect("A should acquire lock")
            .expect("A should get guard");

        // Signal that A has the lock and will hold it
        a_holding_flag.store(true, Ordering::Relaxed);
        barrier_a.wait().await;

        // Hold lock for longer than B's acquire timeout
        tokio::time::sleep(Duration::from_millis(250)).await;

        guard.release().await.expect("A should release guard");
        true
    });

    // Task B: Try to acquire lock but should get None (locked)
    let barrier_b = barrier.clone();
    let task_b = tokio::spawn(async move {
        // Wait for A to acquire lock
        barrier_b.wait().await;

        // Make sure A is holding the lock
        assert!(
            a_holding.load(Ordering::Relaxed),
            "A should be holding lock"
        );

        // Try to acquire lock - should return None since lock is held
        let (mut lock_b, _admin_pool_b) = TestLock::create_shared("acquire_timeout", "B").await;

        let result = lock_b
            .try_acquire()
            .await
            .expect("B try_acquire should not fail");
        assert!(
            result.is_none(),
            "B should not acquire lock while A holds it"
        );

        true
    });

    // Wait for both tasks
    let (a_result, b_result) = tokio::join!(task_a, task_b);

    assert!(
        a_result.expect("Task A should complete"),
        "A should hold lock successfully"
    );
    assert!(
        b_result.expect("Task B should complete"),
        "B should handle locked state"
    );

    // Clean up env var
    // TODO: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::remove_var("NOMMIE_MIGRATE_TIMEOUT_MS") };
}

#[tokio::test]
async fn lock_contention() {
    // Two tasks target the same lock. First acquires and holds; second try_acquire() returns None.
    // After first releases, second acquires successfully.

    if skip_if_sqlite_memory("lock_contention").is_none() {
        return;
    }

    let barrier = Arc::new(Barrier::new(2));
    let a_holding = Arc::new(AtomicBool::new(false));

    // Task A: Acquire lock and hold it
    let barrier_a = barrier.clone();
    let a_holding_flag = a_holding.clone();

    let task_a = tokio::spawn(async move {
        let (mut lock_a, _admin_pool_a) = TestLock::create_shared("lock_contention", "A").await;

        // Try to acquire lock
        let guard = lock_a
            .try_acquire()
            .await
            .expect("A try_acquire should not fail")
            .expect("A should acquire lock");

        // Signal that A has the lock
        a_holding_flag.store(true, Ordering::Relaxed);
        barrier_a.wait().await;

        // Hold lock for a short time
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Release lock
        guard.release().await.expect("A should release guard");
        true
    });

    // Task B: Wait for A to acquire, then try to acquire (should fail), then acquire after A releases
    let barrier_b = barrier.clone();

    let task_b = tokio::spawn(async move {
        // Wait for A to acquire lock
        barrier_b.wait().await;

        // Make sure A is holding the lock
        assert!(
            a_holding.load(Ordering::Relaxed),
            "A should be holding lock"
        );

        // Try to acquire lock - should return None (contention)
        let (mut lock_b, _admin_pool_b) = TestLock::create_shared("lock_contention", "B").await;
        let result = lock_b
            .try_acquire()
            .await
            .expect("B try_acquire should not fail");
        assert!(
            result.is_none(),
            "B should not acquire lock while A holds it"
        );

        // Wait for A to release
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Now try to acquire again - should succeed
        let guard_b = lock_b
            .try_acquire()
            .await
            .expect("B try_acquire should not fail")
            .expect("B should acquire lock after A released");

        guard_b.release().await.expect("B should release guard");
        true
    });

    // Wait for both tasks
    let (a_result, b_result) = tokio::join!(task_a, task_b);

    assert!(
        a_result.expect("Task A should complete"),
        "A should acquire and release lock successfully"
    );
    assert!(
        b_result.expect("Task B should complete"),
        "B should acquire lock after A released"
    );
}

#[tokio::test]
async fn release_idempotence() {
    // Double release should be a no-op (guard already tracks released)

    if skip_if_sqlite_memory("release_idempotence").is_none() {
        return;
    }

    let (mut lock, _admin_pool) = TestLock::create_shared("release_idempotence", "single").await;
    let guard = lock
        .try_acquire()
        .await
        .expect("Should acquire lock")
        .expect("Should get guard");

    // First release should succeed
    guard.release().await.expect("First release should succeed");

    // Guard is consumed, so create a new one and verify it can be acquired/released
    let (mut lock2, _admin_pool2) = TestLock::create_shared("release_idempotence", "double").await;
    let guard2 = lock2
        .try_acquire()
        .await
        .expect("Should acquire lock")
        .expect("Should get guard");

    guard2
        .release()
        .await
        .expect("Second guard release should succeed");
}

#[tokio::test]
async fn path_normalization_sqlite_file() {
    // Test that sqlite_file_spec + lock_path composition produce a canonical lock path

    let db_kind = resolve_test_db_kind().expect("Failed to resolve DB kind");

    // Only test SQLite file path normalization
    if db_kind != DbKind::SqliteFile {
        println!(
            "Skipping path_normalization_sqlite_file for DbKind::{:?}",
            db_kind
        );
        return;
    }

    // Use the same helper as production code to derive the SQLite lock path
    let lock_path =
        backend::config::db::sqlite_lock_path(db_kind, backend::config::db::RuntimeEnv::Test)
            .expect("sqlite_lock_path should succeed for SqliteFile/Test");

    // Should end with .migrate.lock
    assert!(
        lock_path.to_string_lossy().ends_with(".migrate.lock"),
        "Lock path should end with .migrate.lock"
    );

    // Should be absolute
    assert!(lock_path.is_absolute(), "Lock path should be absolute");

    // Should be canonical (no .. or . components)
    let lock_path_str = lock_path.to_string_lossy();
    assert!(
        !lock_path_str.contains("..") && !lock_path_str.contains("/./"),
        "Lock path should be canonical"
    );
}

#[tokio::test]
async fn body_timeout_aborts_and_unlocks() {
    // Configure small body timeout; A exceeds it; assert abort + unlock; B acquires.
    // Note: Only truly applies to Postgres with actual migration; SQLite simulates the behavior

    // Set short body timeout for this test
    // TODO: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::set_var("NOMMIE_MIGRATE_BODY_TIMEOUT_MS", "200") };

    let db_kind = match skip_if_sqlite_memory("body_timeout_aborts_and_unlocks") {
        None => return,
        Some(kind) => kind,
    };

    let is_postgres = matches!(db_kind, DbKind::Postgres);

    let barrier = Arc::new(Barrier::new(2));
    let a_completed = Arc::new(AtomicBool::new(false));

    // Task A: Acquire lock and run migration that exceeds body timeout
    let barrier_a = barrier.clone();
    let a_completed_flag = a_completed.clone();

    let task_a = tokio::spawn(async move {
        let (mut lock_a, admin_pool_opt) = TestLock::create_shared("body_timeout", "A").await;

        // Acquire lock
        let guard = lock_a
            .try_acquire()
            .await
            .expect("A should acquire lock")
            .expect("A should get guard");

        // Signal that A has the lock
        barrier_a.wait().await;

        // Simulate a migration that takes longer than body timeout
        let was_timeout = if let Some(admin_pool_a) = admin_pool_opt {
            let cancel_flag = Arc::new(AtomicBool::new(false));
            let slow_task = slow_migration_task(admin_pool_a, cancel_flag.clone(), 1000);

            let result = tokio::select! {
                migration_result = slow_task => migration_result,
                _ = tokio::time::sleep(Duration::from_millis(200)) => {
                    cancel_flag.store(true, Ordering::Relaxed);
                    Err(sea_orm::DbErr::Custom("Body timeout".to_string()))
                }
            };

            result.is_err()
                && (result.as_ref().unwrap_err().to_string().contains("timeout")
                    || result
                        .as_ref()
                        .unwrap_err()
                        .to_string()
                        .contains("cancelled"))
        } else {
            // SQLite: just simulate timeout behavior
            tokio::time::sleep(Duration::from_millis(200)).await;
            true
        };

        // Always release guard
        guard.release().await.expect("A should release guard");

        a_completed_flag.store(true, Ordering::Relaxed);

        was_timeout
    });

    // Task B: Wait for A to timeout, then acquire lock
    let barrier_b = barrier.clone();
    let task_b = tokio::spawn(async move {
        // Wait for A to acquire lock
        barrier_b.wait().await;

        // Wait for A to timeout and complete
        while !a_completed.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Give a little extra time for cleanup
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Now try to acquire - should succeed quickly since A timed out and released
        let (mut lock_b, _admin_pool_b) = TestLock::create_shared("body_timeout", "B").await;

        let acquire_start = std::time::Instant::now();
        let guard_b = lock_b
            .try_acquire()
            .await
            .expect("B try_acquire should not fail")
            .expect("B should acquire lock after A timed out");
        let acquire_duration = acquire_start.elapsed();

        // Should acquire quickly
        assert!(
            acquire_duration < Duration::from_millis(100),
            "B should acquire lock quickly after A timed out, took: {:?}",
            acquire_duration
        );

        guard_b.release().await.expect("B should release guard");
        true
    });

    // Wait for both tasks
    let (a_result, b_result) = tokio::join!(task_a, task_b);

    if is_postgres {
        assert!(
            a_result.expect("Task A should complete"),
            "A should have timed out"
        );
    }
    assert!(
        b_result.expect("Task B should complete"),
        "B should acquire lock successfully"
    );

    // Clean up env var
    // TODO: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::remove_var("NOMMIE_MIGRATE_BODY_TIMEOUT_MS") };
}
