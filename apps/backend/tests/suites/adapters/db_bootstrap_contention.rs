use backend::config::db::RuntimeEnv;
use backend::infra::db::{bootstrap_db, build_admin_pool};
use futures::future::join_all;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, DatabaseBackend};
use tokio::time::{timeout, Duration};
use {tracing, tracing_subscriber};

use crate::support::resolve_test_db_kind;

#[tokio::test]
async fn contention_burst_all_ok_and_single_migrator() {
    let _ = tracing_subscriber::fmt::try_init();

    let env = RuntimeEnv::Test;
    let db_kind = resolve_test_db_kind().expect("Failed to resolve DB kind");

    // SQLite memory creates separate database instances per connection type
    // (shared pool vs admin pool), so migration counts won't match across connections
    if matches!(db_kind, backend::config::db::DbKind::SqliteMemory) {
        println!(
            "Skipping contention_burst_all_ok_and_single_migrator for DbKind::{:?}",
            db_kind
        );
        return;
    }
    let burst_n = 6;
    let timeout_secs = 6;

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
    let baseline = Migrator::get_applied_migrations(&baseline_admin_pool)
        .await
        .unwrap_or_else(|_| vec![])
        .len();

    // Determine the correct backend for database operations
    let backend = DatabaseBackend::from(db_kind);

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
    let after = Migrator::get_applied_migrations(&after_admin_pool)
        .await
        .unwrap_or_else(|_| vec![])
        .len();

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
    let again_after = Migrator::get_applied_migrations(&again_after_admin_pool)
        .await
        .unwrap_or_else(|_| vec![])
        .len();

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
