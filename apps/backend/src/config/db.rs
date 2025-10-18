use std::env;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Database profile enum for different environments
#[derive(Debug, Clone, PartialEq)]
pub enum DbProfile {
    /// Production database profile
    Prod,
    /// Test database profile - enforces safety rules
    Test,
    /// SQLite in-memory database (ephemeral, fastest)
    InMemory,
    /// SQLite file-based database (persistent, configurable file)
    SqliteFile { file: Option<String> },
}

/// Database owner enum for different access levels
#[derive(Debug, Clone, PartialEq)]
pub enum DbOwner {
    /// Application-level access (limited permissions)
    App,
    /// Owner-level access (full permissions for migrations)
    Owner,
}

/// Builds a database URL from environment variables based on profile and owner
pub fn db_url(profile: DbProfile, owner: DbOwner) -> Result<String, AppError> {
    let host = host()?;
    let port = port()?;
    let db_name = db_name(profile)?;
    let (username, password) = credentials(owner)?;
    Ok(format!(
        "postgresql://{username}:{password}@{host}:{port}/{db_name}"
    ))
}

/// Get database host from environment (defaults to localhost)
fn host() -> Result<String, AppError> {
    Ok(env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()))
}

/// Get database port from environment (defaults to 5432)
fn port() -> Result<String, AppError> {
    Ok(env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string()))
}

/// Get database name based on profile
fn db_name(profile: DbProfile) -> Result<String, AppError> {
    match profile {
        DbProfile::Prod => Ok(must_var("PROD_DB")?),
        DbProfile::Test => {
            let db_name = must_var("TEST_DB")?;
            if !db_name.ends_with("_test") {
                return Err(AppError::config(format!(
                    "Test profile requires database name to end with '_test', but got: '{db_name}'"
                )));
            }
            Ok(db_name)
        }
        DbProfile::InMemory | DbProfile::SqliteFile { .. } => Err(AppError::config(
            "SQLite profiles don't use database names from environment variables",
        )),
    }
}

/// Get database credentials based on owner
fn credentials(owner: DbOwner) -> Result<(String, String), AppError> {
    match owner {
        DbOwner::App => Ok((must_var("APP_DB_USER")?, must_var("APP_DB_PASSWORD")?)),
        DbOwner::Owner => Ok((
            must_var("NOMMIE_OWNER_USER")?,
            must_var("NOMMIE_OWNER_PASSWORD")?,
        )),
    }
}

/// Get required environment variable or return error
fn must_var(name: &str) -> Result<String, AppError> {
    env::var(name)
        .map_err(|_| AppError::config(format!("Required environment variable '{name}' is not set")))
}

/// Canonical, single source of truth for the SQLite file path.
/// - Uses provided `file` if Some
/// - Else `${SQLITE_DB_DIR:-./data/sqlite}/nommie.db`
/// - Creates parent dir
/// - Canonicalizes leniently (parent first if file not created yet)
pub fn sqlite_file_path(profile: &DbProfile) -> Result<PathBuf, AppError> {
    match profile {
        DbProfile::SqliteFile { file } => {
            let raw = match file {
                Some(p) => PathBuf::from(p),
                None => {
                    let db_dir =
                        env::var("SQLITE_DB_DIR").unwrap_or_else(|_| "./data/sqlite".to_string());
                    Path::new(&db_dir).join("nommie.db")
                }
            };
            if let Some(parent) = raw.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    AppError::config(format!("create_dir_all({}): {e}", parent.display()))
                })?;
            }
            canonicalize_lenient(&raw)
        }
        _ => Err(AppError::config(
            "sqlite_file_path called for non-SQLite profile",
        )),
    }
}

fn canonicalize_lenient(p: &Path) -> Result<PathBuf, AppError> {
    match p.canonicalize() {
        Ok(abs) => Ok(abs),
        Err(_) => {
            let parent = p
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .canonicalize()
                .map_err(|e| AppError::config(format!("canonicalize({}): {e}", p.display())))?;
            let file_name = p.file_name().ok_or_else(|| {
                AppError::config(format!("path has no file name: {}", p.display()))
            })?;
            Ok(parent.join(file_name))
        }
    }
}

/// Stable db key for logging/metrics: "sqlite:file:<canonical path>"
pub fn sqlite_db_key(profile: &DbProfile) -> Result<String, AppError> {
    let path = sqlite_file_path(profile)?;
    Ok(format!("sqlite:file:{}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_test_env() {
        env::set_var("PROD_DB", "nommie");
        env::set_var("TEST_DB", "nommie_test");
        env::set_var("APP_DB_USER", "nommie_app");
        env::set_var("APP_DB_PASSWORD", "app_password");
        env::set_var("NOMMIE_OWNER_USER", "nommie_owner");
        env::set_var("NOMMIE_OWNER_PASSWORD", "owner_password");
    }
    fn clear_test_env() {
        for v in [
            "PROD_DB",
            "TEST_DB",
            "APP_DB_USER",
            "APP_DB_PASSWORD",
            "NOMMIE_OWNER_USER",
            "NOMMIE_OWNER_PASSWORD",
            "POSTGRES_HOST",
            "POSTGRES_PORT",
        ] {
            env::remove_var(v);
        }
    }
    #[test]
    fn test_db_url_prod_app() {
        set_test_env();
        let url = db_url(DbProfile::Prod, DbOwner::App).unwrap();
        assert_eq!(
            url,
            "postgresql://nommie_app:app_password@localhost:5432/nommie"
        );
        clear_test_env();
    }
    #[test]
    fn test_db_url_prod_owner() {
        set_test_env();
        let url = db_url(DbProfile::Prod, DbOwner::Owner).unwrap();
        assert_eq!(
            url,
            "postgresql://nommie_owner:owner_password@localhost:5432/nommie"
        );
        clear_test_env();
    }
    #[test]
    fn test_db_url_test_app() {
        set_test_env();
        let url = db_url(DbProfile::Test, DbOwner::App).unwrap();
        assert_eq!(
            url,
            "postgresql://nommie_app:app_password@localhost:5432/nommie_test"
        );
        clear_test_env();
    }
    #[test]
    fn test_db_url_test_owner() {
        set_test_env();
        let url = db_url(DbProfile::Test, DbOwner::Owner).unwrap();
        assert_eq!(
            url,
            "postgresql://nommie_owner:owner_password@localhost:5432/nommie_test"
        );
        clear_test_env();
    }
    #[test]
    fn test_db_url_with_custom_host_port() {
        set_test_env();
        env::set_var("POSTGRES_HOST", "db.example.com");
        env::set_var("POSTGRES_PORT", "5433");
        let url = db_url(DbProfile::Prod, DbOwner::App).unwrap();
        assert_eq!(
            url,
            "postgresql://nommie_app:app_password@db.example.com:5433/nommie"
        );
        env::remove_var("POSTGRES_HOST");
        env::remove_var("POSTGRES_PORT");
        clear_test_env();
    }
    #[test]
    fn test_db_url_test_invalid_name() {
        set_test_env();
        env::set_var("TEST_DB", "nommie_prod");
        let result = db_url(DbProfile::Test, DbOwner::App);
        assert!(result.is_err());
        clear_test_env();
    }
    #[test]
    fn sqlite_helpers_compile() {
        let _ = sqlite_db_key(&DbProfile::SqliteFile {
            file: Some("/tmp/whatever.db".into()),
        });
    }
}
