use backend::config::db::{DbOwner, DbProfile};
use backend::infra::db::bootstrap_db;
use futures::future::join_all;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tokio::time::{timeout, Duration};
use {tracing, tracing_subscriber};

// Helper: count applied migrations via seaql_migrations table.
async fn migration_count(conn: &impl ConnectionTrait, backend: DatabaseBackend) -> i64 {
    conn.query_one(Statement::from_string(
        backend,
        "SELECT COUNT(*) AS cnt FROM seaql_migrations",
    ))
    .await
    .ok()
    .flatten()
    .and_then(|row| row.try_get::<i64>("", "cnt").ok())
    .unwrap_or(0)
}

async fn assert_contention_run_once_then_idempotent(
    make_profile: impl Fn() -> DbProfile + Send + Sync + 'static,
    backend: DatabaseBackend,
    burst_n: usize,
    timeout_secs: u64,
) {
    let _ = tracing_subscriber::fmt::try_init();

    // Compute total migrations known to the migrator
    let total_migrations = Migrator::migrations().len();

    // Read baseline = migration_count(&pool, backend) using a pool from make_profile()
    let baseline_pool = bootstrap_db(make_profile(), DbOwner::App)
        .await
        .expect("baseline pool");
    let baseline = migration_count(&baseline_pool, backend).await;

    // Launch burst_n concurrent tasks, each calling bootstrap_db(make_profile(), DbOwner::App),
    // wrapped in a timeout of timeout_secs. After each completes, assert the pool
    // can execute "SELECT 1" for the given backend.
    let futs = (0..burst_n).map(|_| async {
        timeout(
            Duration::from_secs(timeout_secs),
            bootstrap_db(make_profile(), DbOwner::App),
        )
        .await
    });
    let results = join_all(futs).await;

    for r in results {
        let pool = r.expect("timeout").expect("bootstrap ok");
        pool.execute(Statement::from_string(backend, "SELECT 1"))
            .await
            .expect("usable pool");
    }

    // Read after = migration_count(&fresh_pool, backend)
    let after_pool = bootstrap_db(make_profile(), DbOwner::App)
        .await
        .expect("after pool");
    let after = migration_count(&after_pool, backend).await;

    // Compute:
    //     applied = after - baseline
    //     expected_pending = max(0, total_migrations as i64 - baseline)
    // ASSERT: applied == expected_pending
    let applied = after - baseline;
    let expected_pending = std::cmp::max(0, total_migrations as i64 - baseline);
    assert_eq!(
        applied, expected_pending,
        "applied migrations count mismatch: applied={}, expected_pending={}",
        applied, expected_pending
    );

    // Launch a second identical burst
    let futs2 = (0..burst_n).map(|_| async {
        timeout(
            Duration::from_secs(timeout_secs),
            bootstrap_db(make_profile(), DbOwner::App),
        )
        .await
    });
    let results2 = join_all(futs2).await;

    for r in results2 {
        let pool = r.expect("timeout").expect("bootstrap ok");
        pool.execute(Statement::from_string(backend, "SELECT 1"))
            .await
            .expect("usable pool");
    }

    // Read again_after = migration_count(&fresh_pool, backend)
    let again_after_pool = bootstrap_db(make_profile(), DbOwner::App)
        .await
        .expect("again_after pool");
    let again_after = migration_count(&again_after_pool, backend).await;

    // ASSERT: again_after == after
    assert_eq!(
        again_after, after,
        "Second burst should be idempotent: after={}, again_after={}",
        after, again_after
    );

    // Log with tracing::info! the following (and only these) checkpoints:
    // total_migrations, baseline, after, applied, expected_pending, again_after
    tracing::info!(
        "total_migrations={}, baseline={}, after={}, applied={}, expected_pending={}, again_after={}",
        total_migrations, baseline, after, applied, expected_pending, again_after
    );
}

#[tokio::test]
#[cfg_attr(not(feature = "regression-tests"), ignore)]
async fn pg_contention_burst_all_ok_and_single_migrator() {
    assert_contention_run_once_then_idempotent(
        || DbProfile::Test,
        DatabaseBackend::Postgres,
        6, // burst_n
        6, // timeout_secs
    )
    .await;
}

#[tokio::test]
#[cfg_attr(not(feature = "regression-tests"), ignore)]
async fn sqlite_file_sidecar_lock_under_parallel_bootstrap() {
    // Build one shared file profile to contend on the sidecar lock.
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("db.sqlite3");

    assert_contention_run_once_then_idempotent(
        {
            let path = db_path.clone();
            move || DbProfile::SqliteFile {
                file: Some(path.to_string_lossy().into()),
            }
        },
        DatabaseBackend::Sqlite,
        6, // burst_n
        6, // timeout_secs
    )
    .await;
}
