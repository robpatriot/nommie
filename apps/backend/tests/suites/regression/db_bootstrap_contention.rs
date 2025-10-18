use backend::config::db::{DbOwner, DbProfile};
use backend::infra::db::bootstrap_db;
// Counter shim:
// - When the feature is ON, use the real counter.
// - When OFF, provide no-op stubs so the file still compiles cleanly.
#[cfg(feature = "regression-tests")]
use backend::infra::db::test_infra_counters as cnt;
use futures::future::join_all;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tokio::time::{timeout, Duration};
use tracing;

#[cfg(not(feature = "regression-tests"))]
mod cnt {
    pub fn reset() {}
}

// Helper: count applied migrations in Postgres (via seaql_migrations).
async fn migration_count_pg(conn: &impl ConnectionTrait) -> i64 {
    conn.query_one(Statement::from_string(
        DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS cnt FROM seaql_migrations",
    ))
    .await
    .ok()
    .flatten()
    .and_then(|row| row.try_get::<i64>("", "cnt").ok())
    .unwrap_or(0)
}

#[tokio::test]
#[cfg_attr(not(feature = "regression-tests"), ignore)]
async fn pg_contention_burst_all_ok_and_single_migrator() {
    // Ensure counter starts clean (no-op when feature is off). We will only print it.
    cnt::reset();

    // Compute total migrations known to the migrator
    let total_migrations = Migrator::migrations().len();
    tracing::info!("total_migrations={total_migrations}");

    // Baseline: record the current migration count.
    let baseline = {
        let pool = bootstrap_db(DbProfile::Test, DbOwner::App)
            .await
            .expect("baseline pool");
        migration_count_pg(&pool).await
    };
    tracing::info!("baseline={baseline}");

    // First concurrent burst.
    let n = 6usize;
    tracing::info!("pg_contention: launching {n} concurrent bootstraps");
    let futs = (0..n).map(|_| async {
        timeout(
            Duration::from_secs(6),
            bootstrap_db(DbProfile::Test, DbOwner::App),
        )
        .await
    });
    let results = join_all(futs).await;

    let mut ok = 0usize;
    for r in results {
        let pool = r.expect("timeout").expect("bootstrap ok");
        pool.execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT 1",
        ))
        .await
        .expect("usable pool");
        ok += 1;
    }
    tracing::info!("pg_contention: {ok} pools usable");

    // After first burst.
    let after = {
        let pool = bootstrap_db(DbProfile::Test, DbOwner::App)
            .await
            .expect("post pool");
        migration_count_pg(&pool).await
    };
    tracing::info!("after={after}");

    // Compute exact number of migrations applied during first burst
    let applied = after - baseline;
    let expected_pending = std::cmp::max(0, total_migrations as i64 - baseline);

    tracing::info!("applied={applied}");
    tracing::info!("expected_pending={expected_pending}");

    // Assert: applied == expected_pending
    assert_eq!(
        applied, expected_pending,
        "applied migrations count mismatch"
    );

    // Second concurrent burst: should be a no-op (idempotent under contention).
    let futs2 = (0..n).map(|_| async {
        timeout(
            Duration::from_secs(6),
            bootstrap_db(DbProfile::Test, DbOwner::App),
        )
        .await
    });
    let results2 = join_all(futs2).await;

    let mut ok2 = 0usize;
    for r in results2 {
        let pool = r.expect("timeout").expect("bootstrap ok");
        pool.execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT 1",
        ))
        .await
        .expect("usable pool");
        ok2 += 1;
    }
    tracing::info!("pg_contention (2nd): {ok2} pools usable");

    let again_after = {
        let pool = bootstrap_db(DbProfile::Test, DbOwner::App)
            .await
            .expect("post2 pool");
        migration_count_pg(&pool).await
    };
    tracing::info!("again_after={again_after}");

    // Assert: again_after == after (no extra migrations applied in second burst)
    assert_eq!(
        again_after, after,
        "contention burst applied migrations more than once"
    );
}

#[tokio::test]
#[cfg_attr(not(feature = "regression-tests"), ignore)]
async fn sqlite_file_sidecar_lock_under_parallel_bootstrap() {
    // Build one shared file profile to contend on the sidecar lock.
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("db.sqlite3");
    let profile = DbProfile::SqliteFile {
        file: Some(db_path.to_string_lossy().into()),
    };

    tracing::info!("sqlite_contention: db_path={}", db_path.display());

    let n = 5usize;
    tracing::info!("sqlite_contention: launching {n} concurrent bootstraps");

    let futs = (0..n).map(|_| async {
        timeout(
            Duration::from_secs(6),
            bootstrap_db(profile.clone(), DbOwner::App),
        )
        .await
    });
    let results = join_all(futs).await;

    let mut ok = 0usize;
    for r in results {
        let pool = r.expect("timeout").expect("bootstrap ok");
        pool.execute(Statement::from_string(DatabaseBackend::Sqlite, "SELECT 1"))
            .await
            .expect("usable sqlite pool");
        ok += 1;
    }
    tracing::info!("sqlite_contention: {ok} pools usable");
}
