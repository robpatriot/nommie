use backend::config::db::{DbKind, RuntimeEnv};
use backend::infra::db::{bootstrap_db, build_admin_pool};
use futures::future::join_all;
use migration::{count_applied_migrations, Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, DatabaseBackend};
use tokio::time::{timeout, Duration};
use {tracing, tracing_subscriber};

async fn assert_contention_run_once_then_idempotent(
    env: RuntimeEnv,
    db_kind: DbKind,
    backend: DatabaseBackend,
    burst_n: usize,
    timeout_secs: u64,
) {
    let _ = tracing_subscriber::fmt::try_init();

    // Compute total migrations known to the migrator
    let total_migrations = Migrator::migrations().len();

    // First bootstrap to ensure database is ready
    let _baseline_bootstrap = bootstrap_db(env, db_kind)
        .await
        .expect("baseline bootstrap");

    // Use admin pool for accurate migration count (admin credentials ensure full database access)
    let baseline_admin_pool = build_admin_pool(env, db_kind)
        .await
        .expect("baseline admin pool");
    let baseline = count_applied_migrations(&baseline_admin_pool)
        .await
        .unwrap_or(0);

    // Launch burst_n concurrent tasks, each calling bootstrap_db(env, db_kind),
    // wrapped in a timeout of timeout_secs. After each completes, assert the pool
    // can execute "SELECT 1" for the given backend.
    let futs = (0..burst_n).map(|_| async {
        timeout(
            Duration::from_secs(timeout_secs),
            bootstrap_db(env, db_kind),
        )
        .await
    });
    let results = join_all(futs).await;

    for r in results {
        let pool = r.expect("timeout").expect("bootstrap ok");
        pool.execute(sea_orm::Statement::from_string(backend, "SELECT 1"))
            .await
            .expect("usable pool");
    }

    // Read after = migration_count using admin pool for accurate count
    let after_admin_pool = build_admin_pool(env, db_kind)
        .await
        .expect("after admin pool");
    let after = count_applied_migrations(&after_admin_pool)
        .await
        .unwrap_or(0);

    // Compute:
    //     applied = after - baseline
    //     expected_pending = max(0, total_migrations - baseline)
    // ASSERT: applied == expected_pending
    let applied = after - baseline;
    let expected_pending = std::cmp::max(0, total_migrations - baseline);

    // Handle the case where database already has migrations from previous run
    // In this case, baseline should equal total_migrations (all already applied)
    // and after should also equal total_migrations (idempotent)
    // This means applied = 0 and expected_pending = 0 (nothing pending)
    if baseline >= total_migrations && after >= total_migrations {
        // Database was already fully migrated - all concurrent bootstraps should be idempotent
        assert_eq!(
            applied, 0,
            "applied migrations count mismatch when already migrated: applied={}, baseline={}, after={}, total_migrations={}",
            applied, baseline, after, total_migrations
        );
    } else {
        // Database was not fully migrated - concurrent bootstraps should apply pending migrations
        assert_eq!(
            applied, expected_pending,
            "applied migrations count mismatch: applied={}, expected_pending={}, baseline={}, after={}, total_migrations={}",
            applied, expected_pending, baseline, after, total_migrations
        );
    }

    // Launch a second identical burst
    let futs2 = (0..burst_n).map(|_| async {
        timeout(
            Duration::from_secs(timeout_secs),
            bootstrap_db(env, db_kind),
        )
        .await
    });
    let results2 = join_all(futs2).await;

    for r in results2 {
        let pool = r.expect("timeout").expect("bootstrap ok");
        pool.execute(sea_orm::Statement::from_string(backend, "SELECT 1"))
            .await
            .expect("usable pool");
    }

    // Read again_after = migration_count using admin pool for accurate count
    let again_after_admin_pool = build_admin_pool(env, db_kind)
        .await
        .expect("again_after admin pool");
    let again_after = count_applied_migrations(&again_after_admin_pool)
        .await
        .unwrap_or(0);

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
        RuntimeEnv::Test,
        DbKind::Postgres,
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
    assert_contention_run_once_then_idempotent(
        RuntimeEnv::Test,
        DbKind::SqliteFile,
        DatabaseBackend::Sqlite,
        6, // burst_n
        6, // timeout_secs
    )
    .await;
}
