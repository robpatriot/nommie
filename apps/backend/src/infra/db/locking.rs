// Standard library imports
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// External crate imports
use async_trait::async_trait;
use futures::Future;
use rand::Rng;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use tracing::{debug, warn};
use xxhash_rust::xxh3::xxh3_64;

// Internal crate imports
use crate::error::AppError;

/// Migration timing constants - explicit and overridable via environment.
const BACKOFF_JITTER_MS_MAX: u64 = 3;

pub fn pg_lock_id(key: &str) -> i64 {
    xxh3_64(key.as_bytes()) as i64
}

// ============================================================================
// BootstrapLock Trait and Implementations
// ============================================================================

/// Guard struct that represents a held lock.
/// Only holds (admin-pool handle, lock key, released flag) - no long-lived checkout.
/// For SQLite file locks, holds the OS file handle directly.
pub struct Guard {
    admin_pool: Option<DatabaseConnection>,
    lock_key: i64,
    sqlite_file: Option<File>, // OS file handle for SQLite file locks
    sqlite_lock_path: Option<PathBuf>, // Path for logging/debugging
    released: bool,
}

impl Guard {
    /// Create a new Postgres guard
    fn postgres(admin_pool: DatabaseConnection, lock_key: i64) -> Self {
        Self {
            admin_pool: Some(admin_pool),
            lock_key,
            sqlite_file: None,
            sqlite_lock_path: None,
            released: false,
        }
    }

    /// Create a new SQLite file lock guard
    fn sqlite(file: File, lock_path: PathBuf) -> Self {
        Self {
            admin_pool: None,
            lock_key: -1, // Sentinel value for SQLite file locks
            sqlite_file: Some(file),
            sqlite_lock_path: Some(lock_path),
            released: false,
        }
    }

    /// Create a new InMemory no-op guard
    fn in_memory() -> Self {
        Self {
            admin_pool: None,
            lock_key: 0, // Sentinel value for InMemory
            sqlite_file: None,
            sqlite_lock_path: None,
            released: false,
        }
    }

    /// Release the lock by checking out from admin pool, unlocking, then dropping checkout.
    pub async fn release(mut self) -> Result<(), AppError> {
        if self.released {
            return Ok(());
        }

        // Handle SQLite file locks
        if let Some(file) = self.sqlite_file.take() {
            let lock_path_display = self
                .sqlite_lock_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            match file.unlock() {
                Ok(()) => {
                    debug!(lock_path = lock_path_display, "SQLite file lock released");
                }
                Err(e) => {
                    // Unlock errors are usually benign (e.g., already unlocked, file closed)
                    // Log as debug, not warning, since the file will be dropped anyway
                    debug!(
                        error = %e,
                        lock_path = lock_path_display,
                        "SQLite file unlock returned error (may be benign)"
                    );
                }
            }
            // File is dropped here, releasing OS-level lock
            self.released = true;
            return Ok(());
        }

        // Handle InMemory guards (no admin_pool and no sqlite_file means InMemory no-op guard)
        if self.admin_pool.is_none() {
            self.released = true;
            return Ok(());
        }

        // Handle Postgres locks
        let Some(admin_pool) = &self.admin_pool else {
            self.released = true;
            return Ok(());
        };

        use sea_orm::{DatabaseBackend, Statement};

        // Execute unlock query using SeaORM
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_advisory_unlock($1) AS unlocked",
            vec![self.lock_key.into()],
        );

        let result = admin_pool.query_one(stmt).await;

        match result {
            Ok(Some(row)) => {
                let unlocked: bool = row
                    .try_get("", "unlocked")
                    .map_err(|e| AppError::config("failed to read unlock result", e))?;

                if !unlocked {
                    warn!(
                        code = "PG_UNLOCK_FALSE",
                        lock_key = self.lock_key,
                        "Advisory lock unlock returned false"
                    );
                }
            }
            Ok(None) => {
                warn!(
                    lock_key = self.lock_key,
                    "No result from advisory lock unlock query"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    lock_key = self.lock_key,
                    "Failed to unlock advisory lock"
                );
            }
        }

        self.released = true;
        Ok(())
    }
}

/// Trait for bootstrap/migration lock acquisition and release.
/// Abstracts over PostgreSQL advisory locks and SQLite file locks.
#[async_trait]
pub trait BootstrapLock {
    /// Try to acquire the lock (non-blocking).
    /// Returns Some(Guard) if acquired, None if already held by another process.
    async fn try_acquire(&mut self) -> Result<Option<Guard>, AppError>;
}

/// PostgreSQL advisory lock using admin pool
pub struct PgAdvisoryLock {
    admin_pool: DatabaseConnection,
    lock_key: i64,
}

impl PgAdvisoryLock {
    /// Create a new PostgreSQL advisory lock.
    /// Uses admin pool for lock operations.
    ///
    /// INVARIANT: This code assumes the admin pool is configured with **min=max=1**
    /// so all checkouts reuse the **same** physical session that holds the advisory lock.
    /// If this invariant changes, the locking strategy must be revisited.
    pub fn new(admin_pool: DatabaseConnection, key: &str) -> Self {
        let lock_key = pg_lock_id(key);

        Self {
            admin_pool,
            lock_key,
        }
    }
}

#[async_trait]
impl BootstrapLock for PgAdvisoryLock {
    async fn try_acquire(&mut self) -> Result<Option<Guard>, AppError> {
        use sea_orm::{DatabaseBackend, Statement};

        // Step 1: Try to acquire advisory lock
        let lock_stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_try_advisory_lock($1) AS locked",
            vec![self.lock_key.into()],
        );

        let result = self
            .admin_pool
            .query_one(lock_stmt)
            .await
            .map_err(|e| AppError::config("failed to acquire advisory lock", e))?;

        let locked: bool = match result {
            Some(row) => row
                .try_get("", "locked")
                .map_err(|e| AppError::config("failed to read lock result", e))?,
            None => {
                return Err(AppError::config_msg(
                    "advisory lock query returned no result",
                    "pg_try_advisory_lock returned no row",
                ))
            }
        };

        if !locked {
            // Lock not acquired
            return Ok(None);
        }

        // Step 2: Return Guard (no long-lived checkout)
        // Note: Session settings are now applied in migrate_with_guard_controlled
        Ok(Some(Guard::postgres(
            self.admin_pool.clone(),
            self.lock_key,
        )))
    }
}

// PgAdvisoryLock has no additional inherent methods; all behavior is via trait impls

/// SQLite file lock implementation using OS-level exclusive file locks.
/// Uses an OS-level exclusive file lock on `<db>.migrate.lock` for mutual exclusion across processes.
/// Non-blocking `try_lock_exclusive()` integrates with our backoff/timeout loop.
pub struct SqliteFileLock {
    lock_path: PathBuf,
}

impl SqliteFileLock {
    /// Create a new SQLite file lock.
    /// Takes a normalized lock file path (all processes must resolve the same on-disk lock file).
    pub fn new(lock_path: &Path) -> Result<Self, AppError> {
        Ok(Self {
            lock_path: lock_path.to_path_buf(),
        })
    }

    /// Normalize a SQLite DSN or file path to a canonical lock file path.
    /// All processes must resolve the same on-disk lock file for mutual exclusion.
    pub fn normalize_lock_path(dsn_or_path: &str) -> Result<PathBuf, AppError> {
        // Handle in-memory (shouldn't be called, but defensive)
        if dsn_or_path.contains(":memory:") {
            return Err(AppError::config_msg(
                "in-memory database not supported",
                "Cannot create file lock for in-memory database",
            ));
        }

        // Strip sqlite:// prefix if present
        let file_path = dsn_or_path
            .strip_prefix("sqlite://")
            .unwrap_or(dsn_or_path)
            .strip_prefix("sqlite:")
            .unwrap_or(dsn_or_path);

        let db_path = Path::new(file_path);

        // Derive lock file path: <db>.migrate.lock
        let lock_path = if let Some(parent) = db_path.parent() {
            let stem = db_path
                .file_stem()
                .unwrap_or_else(|| std::ffi::OsStr::new("nommie"));
            parent.join(format!("{}.migrate.lock", stem.to_string_lossy()))
        } else {
            // No parent directory, use current directory
            let stem = db_path
                .file_stem()
                .unwrap_or_else(|| std::ffi::OsStr::new("nommie"));
            PathBuf::from(format!("{}.migrate.lock", stem.to_string_lossy()))
        };

        // Resolve to absolute path for consistency across processes
        let canonical = if lock_path.exists() {
            std::fs::canonicalize(&lock_path).map_err(|e| {
                AppError::internal(
                    crate::errors::ErrorCode::ConfigError,
                    "failed to canonicalize lock path",
                    e,
                )
            })?
        } else {
            // File doesn't exist yet, canonicalize parent and join filename
            let parent = lock_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .canonicalize()
                .map_err(|e| {
                    AppError::internal(
                        crate::errors::ErrorCode::ConfigError,
                        "failed to canonicalize lock parent directory",
                        e,
                    )
                })?;
            let filename = lock_path.file_name().ok_or_else(|| {
                AppError::config_msg("lock path has no filename", "lock path has no filename")
            })?;
            parent.join(filename)
        };

        Ok(canonical)
    }
}

#[async_trait]
impl BootstrapLock for SqliteFileLock {
    async fn try_acquire(&mut self) -> Result<Option<Guard>, AppError> {
        use fs4::fs_std::FileExt;

        // Ensure parent directory exists
        if let Some(parent) = self.lock_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::internal(
                    crate::errors::ErrorCode::SqliteLockError,
                    "failed to create lock file parent directory",
                    e,
                )
            })?;
        }

        // Open the lock file (create if doesn't exist)
        // Lock files are ephemeral - truncate on create to ensure clean state
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&self.lock_path)
            .map_err(|e| {
                AppError::internal(
                    crate::errors::ErrorCode::SqliteLockError,
                    "failed to open lock file",
                    e,
                )
            })?;

        // Try to acquire exclusive lock (non-blocking)
        // fs4::FileExt::try_lock_exclusive() returns io::Result<bool>
        // where Ok(true) = lock acquired, Ok(false) = would block, Err = I/O error
        match file.try_lock_exclusive() {
            Ok(true) => {
                // Lock acquired successfully
                debug!(
                    lock_path = %self.lock_path.display(),
                    "SQLite file lock acquired"
                );
                Ok(Some(Guard::sqlite(file, self.lock_path.clone())))
            }
            Ok(false) => {
                // Lock is held by another process - return None (contention)
                debug!(
                    lock_path = %self.lock_path.display(),
                    "SQLite file lock contended (would block)"
                );
                Ok(None)
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Also handle WouldBlock error variant (defensive)
                debug!(
                    lock_path = %self.lock_path.display(),
                    "SQLite file lock contended (would block error)"
                );
                Ok(None)
            }
            Err(e) => {
                // Other I/O error (permission denied, etc.)
                Err(AppError::internal(
                    crate::errors::ErrorCode::SqliteLockError,
                    "failed to acquire SQLite file lock",
                    e,
                ))
            }
        }
    }
}

/// No-op lock implementation for InMemory databases.
/// InMemory databases don't need locking since they're single-process.
pub struct InMemoryLock;

#[async_trait]
impl BootstrapLock for InMemoryLock {
    async fn try_acquire(&mut self) -> Result<Option<Guard>, AppError> {
        // InMemory databases don't need locking - return a no-op Guard
        // We need to return Some(Guard) so the migration system doesn't timeout

        Ok(Some(Guard::in_memory()))
    }
}

/// Backoff loop with exponential backoff and jitter for lock contention.
/// Continuously checks the provided function until it returns true or timeout is reached.
pub async fn backoff_loop<F, Fut>(mut check_fn: F, timeout_ms: u64) -> Result<(), AppError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<bool, AppError>>,
{
    let start = Instant::now();
    let mut attempts: u32 = 0;

    loop {
        attempts += 1;

        // Check timeout
        if start.elapsed() >= Duration::from_millis(timeout_ms) {
            return Err(AppError::config(
                "migration lock acquisition timeout",
                std::io::Error::other(format!(
                    "migration lock timeout after {:?} ({} attempts)",
                    start.elapsed(),
                    attempts
                )),
            ));
        }

        // Backoff with jitter
        let base_delay_ms = (5u64 << attempts.saturating_sub(1)).min(80);
        let jitter_ms = rand::thread_rng().gen_range(0..=BACKOFF_JITTER_MS_MAX);
        let delay_ms = base_delay_ms + jitter_ms;

        debug!(
            lock = "backoff",
            attempts = attempts,
            delay_ms = delay_ms,
            elapsed_ms = start.elapsed().as_millis()
        );

        tokio::time::sleep(Duration::from_millis(delay_ms)).await;

        // Fast-path check
        if check_fn().await? {
            debug!(fastpath = "hit", "Schema became ready during backoff");
            return Ok(());
        }
    }
}
