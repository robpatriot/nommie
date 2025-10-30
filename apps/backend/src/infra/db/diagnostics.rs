// No top-level imports needed - all imports are within the modules

/// SQLite diagnostics and connection tracking
pub mod sqlite_diagnostics {
    use std::process;
    use std::time::Instant;

    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    use tracing::info;

    use crate::error::AppError;

    /// Generate a short random hex ID for pool/connection tracking
    pub fn generate_short_id() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: u32 = rng.gen();
        format!("{:08x}", bytes)[..8].to_string()
    }

    /// Get current process ID
    pub fn current_pid() -> u32 {
        process::id()
    }

    /// Generate a consistent connection-based identifier for logging correlation
    /// Uses readily available connection properties: PID + backend + connection pointer
    pub fn connection_id<C: ConnectionTrait>(conn: &C) -> String {
        let pid = current_pid();
        let backend = match conn.get_database_backend() {
            DatabaseBackend::Sqlite => "sq",
            DatabaseBackend::Postgres => "pg",
            DatabaseBackend::MySql => "my",
        };
        let conn_ptr = conn as *const _ as usize;
        format!("{}-{}-{:x}", pid, backend, conn_ptr % 0xFFFF) // Keep last 4 hex digits
    }

    /// Log PRAGMA values for SQLite diagnostics
    pub async fn log_pragma_snapshot<C: ConnectionTrait>(
        conn: &C,
        pool_type: &str,
    ) -> Result<(), AppError> {
        let pid = current_pid();

        // Infer database type from connection
        let db_type = match conn.get_database_backend() {
            DatabaseBackend::Sqlite => "sqlite",
            DatabaseBackend::Postgres => "postgres",
            DatabaseBackend::MySql => {
                return Err(AppError::config_msg(
                    "MySQL not supported",
                    "MySQL database backend is not supported, only SQLite and PostgreSQL are supported",
                ));
            }
        };

        let pragmas = [
            "journal_mode",
            "synchronous",
            "busy_timeout",
            "locking_mode",
            "mmap_size",
        ];

        for pragma in &pragmas {
            let query = format!("PRAGMA {};", pragma);
            if let Ok(Some(row)) = conn
                .query_one(Statement::from_string(DatabaseBackend::Sqlite, query))
                .await
            {
                // Try different types that this PRAGMA might return
                let formatted_value = row
                    .try_get::<String>("", pragma)
                    .ok()
                    .or_else(|| row.try_get::<i32>("", pragma).ok().map(|v| v.to_string()))
                    .or_else(|| row.try_get::<i64>("", pragma).ok().map(|v| v.to_string()));

                if let Some(value) = formatted_value {
                    info!(
                        pid = pid,
                        db_type = db_type,
                        pool_type = pool_type,
                        pragma = pragma,
                        value = value,
                        "PRAGMA snapshot"
                    );
                }
            }
        }

        Ok(())
    }

    /// Probe SQLite lock acquisition
    pub async fn sqlite_lock_probe<C: ConnectionTrait>(
        conn: &C,
        pool_id: &str,
    ) -> Result<(), AppError> {
        let pid = current_pid();
        let start = Instant::now();

        let result = conn
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "BEGIN IMMEDIATE; ROLLBACK;".to_string(),
            ))
            .await;
        let elapsed_ms = start.elapsed().as_millis();

        match result {
            Ok(_) => {
                info!(
                    pid = pid,
                    pool_id = pool_id,
                    elapsed_ms = elapsed_ms,
                    acquired = true,
                    "SQLite lock probe success"
                );
                Ok(())
            }
            Err(e) => {
                info!(pid=pid, pool_id=pool_id, elapsed_ms=elapsed_ms, acquired=false, error=%e, "SQLite lock probe failed");
                Err(crate::infra::db::core::config_error_with_context(
                    "SQLite lock probe failed",
                    e,
                ))
            }
        }
    }

    /// Redact SQL statement for logging (replace values with ?)
    pub fn redact_sql_preview(sql: &str) -> String {
        // Basic redaction - replace quoted strings and numbers with ?
        let mut redacted = sql.to_string();

        // Replace single-quoted strings
        let mut chars: Vec<char> = redacted.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '\'' {
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                if i < chars.len() {
                    for item in chars.iter_mut().take(i).skip(start + 1) {
                        *item = '?';
                    }
                }
            }
            i += 1;
        }

        // Replace double-quoted strings
        let redacted_str: String = chars.into_iter().collect();
        redacted = redacted_str;

        let mut chars: Vec<char> = redacted.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '"' {
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    i += 1;
                }
                if i < chars.len() {
                    for item in chars.iter_mut().take(i).skip(start + 1) {
                        *item = '?';
                    }
                }
            }
            i += 1;
        }

        let result: String = chars.into_iter().collect();
        result.chars().take(80).collect()
    }
}

/// Migration counters - module-local atomics
pub mod migration_counters {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SCHEMA_CHECKS_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static MIGRATOR_RAN_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static BUSY_EVENTS_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static FAST_PATH_HIT_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static FAST_PATH_MISS_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static LOCK_ACQUIRE_ATTEMPTS_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static LOCK_BACKOFF_EVENTS_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static LOCK_ACQUIRED_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static LOCK_ACQUIRE_TIMEOUTS_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static MIGRATION_BODY_TIMEOUTS_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static MIGRATION_CANCELLED_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static MIGRATION_FAILED_TOTAL: AtomicUsize = AtomicUsize::new(0);
    static POSTCHECK_MISMATCH_TOTAL: AtomicUsize = AtomicUsize::new(0);

    pub fn schema_check() {
        SCHEMA_CHECKS_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn migrator_ran() {
        MIGRATOR_RAN_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn busy_event() {
        BUSY_EVENTS_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn fast_path_hit() {
        FAST_PATH_HIT_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn fast_path_miss() {
        FAST_PATH_MISS_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_lock_acquire_attempts(n: usize) {
        LOCK_ACQUIRE_ATTEMPTS_TOTAL.fetch_add(n, Ordering::Relaxed);
    }

    pub fn lock_backoff_event() {
        LOCK_BACKOFF_EVENTS_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn lock_acquired() {
        LOCK_ACQUIRED_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn lock_acquire_timeout() {
        LOCK_ACQUIRE_TIMEOUTS_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn migration_body_timeout() {
        MIGRATION_BODY_TIMEOUTS_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn migration_cancelled() {
        MIGRATION_CANCELLED_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn migration_failed() {
        MIGRATION_FAILED_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    pub fn postcheck_mismatch() {
        POSTCHECK_MISMATCH_TOTAL.fetch_add(1, Ordering::Relaxed);
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Snapshot {
        pub schema_checks_total: usize,
        pub migrator_ran_total: usize,
        pub busy_events_total: usize,
        pub fast_path_hit_total: usize,
        pub fast_path_miss_total: usize,
        pub lock_acquire_attempts_total: usize,
        pub lock_backoff_events_total: usize,
        pub lock_acquired_total: usize,
        pub lock_acquire_timeouts_total: usize,
        pub migration_body_timeouts_total: usize,
        pub migration_cancelled_total: usize,
        pub migration_failed_total: usize,
        pub postcheck_mismatch_total: usize,
    }

    pub fn snapshot() -> Snapshot {
        Snapshot {
            schema_checks_total: SCHEMA_CHECKS_TOTAL.load(Ordering::Relaxed),
            migrator_ran_total: MIGRATOR_RAN_TOTAL.load(Ordering::Relaxed),
            busy_events_total: BUSY_EVENTS_TOTAL.load(Ordering::Relaxed),
            fast_path_hit_total: FAST_PATH_HIT_TOTAL.load(Ordering::Relaxed),
            fast_path_miss_total: FAST_PATH_MISS_TOTAL.load(Ordering::Relaxed),
            lock_acquire_attempts_total: LOCK_ACQUIRE_ATTEMPTS_TOTAL.load(Ordering::Relaxed),
            lock_backoff_events_total: LOCK_BACKOFF_EVENTS_TOTAL.load(Ordering::Relaxed),
            lock_acquired_total: LOCK_ACQUIRED_TOTAL.load(Ordering::Relaxed),
            lock_acquire_timeouts_total: LOCK_ACQUIRE_TIMEOUTS_TOTAL.load(Ordering::Relaxed),
            migration_body_timeouts_total: MIGRATION_BODY_TIMEOUTS_TOTAL.load(Ordering::Relaxed),
            migration_cancelled_total: MIGRATION_CANCELLED_TOTAL.load(Ordering::Relaxed),
            migration_failed_total: MIGRATION_FAILED_TOTAL.load(Ordering::Relaxed),
            postcheck_mismatch_total: POSTCHECK_MISMATCH_TOTAL.load(Ordering::Relaxed),
        }
    }

    pub fn log_snapshot(context: &str) {
        let s = snapshot();
        tracing::info!(
            context = context,
            schema_checks_total = s.schema_checks_total,
            migrator_ran_total = s.migrator_ran_total,
            busy_events_total = s.busy_events_total,
            fast_path_hit_total = s.fast_path_hit_total,
            fast_path_miss_total = s.fast_path_miss_total,
            lock_acquire_attempts_total = s.lock_acquire_attempts_total,
            lock_backoff_events_total = s.lock_backoff_events_total,
            lock_acquired_total = s.lock_acquired_total,
            lock_acquire_timeouts_total = s.lock_acquire_timeouts_total,
            migration_body_timeouts_total = s.migration_body_timeouts_total,
            migration_cancelled_total = s.migration_cancelled_total,
            migration_failed_total = s.migration_failed_total,
            postcheck_mismatch_total = s.postcheck_mismatch_total,
            "db_migration_counters_snapshot"
        );
    }
}
