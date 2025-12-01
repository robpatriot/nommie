use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, process};

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use sea_orm::DatabaseBackend;

use crate::error::AppError;

/// Runtime environment enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeEnv {
    /// Production environment
    Prod,
    /// Test environment
    Test,
}

/// Database kind enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DbKind {
    /// PostgreSQL database
    Postgres,
    /// SQLite file-based database
    SqliteFile,
    /// SQLite in-memory database
    SqliteMemory,
}

impl FromStr for DbKind {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "postgres" | "pg" => Ok(DbKind::Postgres),
            "sqlitefile" | "sqlite_file" | "sqlite-file" => Ok(DbKind::SqliteFile),
            "sqlitememory" | "sqlite_memory" | "sqlite-memory" | "sqlite-mem" | "sqlite:memory" => {
                Ok(DbKind::SqliteMemory)
            }
            other => Err(AppError::config_msg(
                format!("unknown database kind: {other}"),
                "invalid database kind",
            )),
        }
    }
}

impl From<DbKind> for DatabaseBackend {
    fn from(db_kind: DbKind) -> Self {
        match db_kind {
            DbKind::Postgres => DatabaseBackend::Postgres,
            DbKind::SqliteFile | DbKind::SqliteMemory => DatabaseBackend::Sqlite,
        }
    }
}

/// Pool purpose enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolPurpose {
    /// Application runtime pools
    Runtime,
    /// Migration/admin pools
    Migration,
}

/// Connection settings - pool-level and database-specific per-connection settings
#[derive(Debug, Clone)]
pub struct ConnectionSettings {
    // Pool-level settings (common to all databases)
    pub pool_min: u32,
    pub pool_max: u32,
    pub acquire_timeout_ms: u64,
    // Database-specific per-connection settings
    pub db_settings: DbSettings,
}

/// Database-specific per-connection settings
#[derive(Debug, Clone)]
pub enum DbSettings {
    Sqlite {
        busy_timeout_ms: u32,
    },
    Postgres {
        app_name: String,
        statement_timeout: String,
        idle_in_transaction_timeout: String,
        lock_timeout: Option<String>,
    },
}

/// Calculate baseline parallelism for the given environment
fn calculate_baseline_parallelism(env: RuntimeEnv) -> Result<u32, AppError> {
    match env {
        RuntimeEnv::Prod => {
            // Use physical CPU count for production
            Ok(num_cpus::get_physical() as u32)
        }
        RuntimeEnv::Test => {
            // Use available parallelism capped at 4 for nextest parallel execution
            let available = std::thread::available_parallelism()
                .map(|n| n.get() as u32)
                .unwrap_or(1);
            Ok(available.min(4))
        }
    }
}

/// Build connection settings from environment, database kind, and pool purpose
pub fn build_connection_settings(
    env: RuntimeEnv,
    db_kind: DbKind,
    purpose: PoolPurpose,
) -> Result<ConnectionSettings, AppError> {
    let baseline = calculate_baseline_parallelism(env)?;

    // Calculate pool_max and acquire_timeout_ms based on env + db_kind, then adjust for purpose
    let (pool_min, pool_max, acquire_timeout_ms, db_settings) = match (env, db_kind) {
        (RuntimeEnv::Prod, DbKind::Postgres) => {
            let pool_max = (baseline + 2).clamp(8, 16);
            let db_settings = DbSettings::Postgres {
                app_name: format!(
                    "nommie-prod-{}-{}",
                    if matches!(purpose, PoolPurpose::Migration) {
                        "migrate"
                    } else {
                        "pool"
                    },
                    std::process::id()
                ),
                statement_timeout: "20000ms".to_string(),
                idle_in_transaction_timeout: "120000ms".to_string(),
                lock_timeout: Some("15000ms".to_string()),
            };
            (2, pool_max, 5000, db_settings)
        }
        (RuntimeEnv::Test, DbKind::Postgres) => {
            let pool_max = (baseline + 2).clamp(8, 16);
            let db_settings = DbSettings::Postgres {
                app_name: format!(
                    "nommie-test-{}-{}",
                    if matches!(purpose, PoolPurpose::Migration) {
                        "migrate"
                    } else {
                        "pool"
                    },
                    std::process::id()
                ),
                statement_timeout: "20000ms".to_string(),
                idle_in_transaction_timeout: "120000ms".to_string(),
                lock_timeout: Some("5000ms".to_string()),
            };
            (4, pool_max, 5000, db_settings)
        }
        (RuntimeEnv::Prod, DbKind::SqliteFile) => {
            let pool_max = (baseline + 2).clamp(8, 16);
            let db_settings = DbSettings::Sqlite {
                busy_timeout_ms: 15000,
            };
            (2, pool_max, 2000, db_settings)
        }
        (RuntimeEnv::Test, DbKind::SqliteFile) => {
            let db_settings = DbSettings::Sqlite {
                busy_timeout_ms: 15000,
            };
            (2, 4, 2000, db_settings)
        }
        (RuntimeEnv::Test, DbKind::SqliteMemory) => {
            // busy_timeout differs by purpose
            let busy_timeout_ms = match purpose {
                PoolPurpose::Runtime => 15000,
                PoolPurpose::Migration => 500,
            };
            let db_settings = DbSettings::Sqlite { busy_timeout_ms };
            (1, 1, 2000, db_settings)
        }
        (RuntimeEnv::Prod, DbKind::SqliteMemory) => {
            return Err(AppError::config_msg(
                "production cannot use SQLite in-memory database",
                "SQLite in-memory database is not allowed in production environment",
            ));
        }
    };

    Ok(ConnectionSettings {
        pool_min,
        pool_max,
        acquire_timeout_ms,
        db_settings,
    })
}

/// Encode username/password for use in PostgreSQL connection URLs.
///
/// Based on RFC 3986 userinfo, allowing unreserved characters plus a
/// conservative subset and percent-encoding everything else.
const USERINFO_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

fn encode_userinfo(value: &str) -> String {
    utf8_percent_encode(value, USERINFO_ENCODE_SET).to_string()
}

/// Validate database configuration
pub fn validate_db_config(env: RuntimeEnv, db_kind: DbKind) -> Result<(), AppError> {
    match (env, db_kind) {
        (RuntimeEnv::Prod, DbKind::SqliteMemory) => Err(AppError::config_msg(
            "production cannot use SQLite in-memory database",
            "SQLite in-memory database is not allowed in production environment",
        )),
        _ => Ok(()),
    }
}

/// Get SQLite file specification for the given database kind
pub fn sqlite_file_spec(db_kind: DbKind, env: RuntimeEnv) -> Result<String, AppError> {
    match db_kind {
        DbKind::SqliteFile => {
            // Inline sqlite_file_path logic
            let db_name = match env {
                RuntimeEnv::Prod => "nommie.db",
                RuntimeEnv::Test => "nommie-test.db",
            };
            let raw = env::temp_dir().join("nommie-db").join(db_name);

            // Create parent directories
            if let Some(parent) = raw.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| AppError::config("failed to create database directory", e))?;
            }

            // Canonicalize the path
            let canonical = canonicalize_lenient(&raw)?;

            // Apply test isolation if enabled
            let path = if env::var("NOMMIE_SQLITE_TEST_ISOLATE")
                .map(|val| val == "1" || val.to_lowercase() == "true")
                .unwrap_or(false)
            {
                let pid = process::id();

                if let Some(extension) = canonical.extension() {
                    let stem = canonical
                        .file_stem()
                        .unwrap_or(std::ffi::OsStr::new("nommie"));
                    let new_filename = format!(
                        "{}-{}.{}",
                        stem.to_string_lossy(),
                        pid,
                        extension.to_string_lossy()
                    );
                    let mut path = canonical.clone();
                    path.pop();
                    path.push(new_filename);
                    path
                } else {
                    // No extension, just append the suffix
                    let new_filename = format!("{}-{}", canonical.display(), pid);
                    PathBuf::from(new_filename)
                }
            } else {
                canonical
            };

            Ok(path.to_string_lossy().to_string())
        }
        _ => Err(AppError::config_msg(
            "sqlite file specification not available",
            "sqlite_file_spec only works with SqliteFile database kind",
        )),
    }
}

/// Create connection string from environment, database kind, and database owner
pub fn make_conn_spec(
    env: RuntimeEnv,
    db_kind: DbKind,
    owner: DbOwner,
) -> Result<String, AppError> {
    match db_kind {
        DbKind::Postgres => {
            // Inline db_url logic
            let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
            let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());

            // Inline db_name logic
            let db_name = match env {
                RuntimeEnv::Prod => must_var("PROD_DB")?,
                RuntimeEnv::Test => {
                    let db_name = must_var("TEST_DB")?;
                    if !db_name.ends_with("_test") {
                        return Err(AppError::config_msg(
                            "test database name invalid",
                            "test database name must end with _test",
                        ));
                    }
                    db_name
                }
            };

            // Inline credentials logic
            let (username_raw, password_raw) = match owner {
                DbOwner::App => (must_var("APP_DB_USER")?, must_var("APP_DB_PASSWORD")?),
                DbOwner::Owner => (
                    must_var("NOMMIE_OWNER_USER")?,
                    must_var("NOMMIE_OWNER_PASSWORD")?,
                ),
            };

            let username = encode_userinfo(&username_raw);
            let password = encode_userinfo(&password_raw);

            let url = format!("postgresql://{username}:{password}@{host}:{port}/{db_name}");
            Ok(url)
        }
        DbKind::SqliteFile => {
            let file_spec = sqlite_file_spec(db_kind, env)?;
            Ok(format!("sqlite:{}?mode=rwc", file_spec))
        }
        DbKind::SqliteMemory => Ok("sqlite::memory:".to_string()),
    }
}

/// Database owner enum for different access levels
#[derive(Debug, Clone, PartialEq)]
pub enum DbOwner {
    /// Application-level access (limited permissions)
    App,
    /// Owner-level access (full permissions for migrations)
    Owner,
}

/// Get required environment variable or return error
fn must_var(name: &str) -> Result<String, AppError> {
    env::var(name).map_err(|e| AppError::config("environment variable missing", e))
}

fn canonicalize_lenient(p: &Path) -> Result<PathBuf, AppError> {
    match p.canonicalize() {
        Ok(abs) => Ok(abs),
        Err(_) => {
            let parent = p
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .canonicalize()
                .map_err(|e| AppError::config("failed to canonicalize path", e))?;
            let file_name = p.file_name().ok_or_else(|| {
                AppError::config_msg("path has no file name", "path has no file name")
            })?;
            Ok(parent.join(file_name))
        }
    }
}
