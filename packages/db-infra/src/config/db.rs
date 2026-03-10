use std::env;

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use tracing::trace;

use crate::error::DbInfraError;

/// Runtime environment enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeEnv {
    /// Production environment
    Prod,
    /// Test environment
    ///
    /// This variant is only constructed in test code, but must be handled in production
    /// match statements since `StateBuilder` can receive it as a parameter.
    #[allow(dead_code)]
    Test,
}

/// Pool purpose enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolPurpose {
    /// Application runtime pools
    Runtime,
    /// Migration/admin pools
    Migration,
}

/// Connection settings - pool-level and per-connection settings for Postgres
#[derive(Debug, Clone)]
pub struct ConnectionSettings {
    // Pool-level settings
    pub pool_min: u32,
    pub pool_max: u32,
    pub acquire_timeout_ms: u64,
    // Database-specific per-connection settings
    pub db_settings: DbSettings,
}

/// Database-specific per-connection settings (Postgres only)
#[derive(Debug, Clone)]
pub struct DbSettings {
    pub app_name: String,
    pub statement_timeout: String,
    pub idle_in_transaction_timeout: String,
    pub lock_timeout: Option<String>,
}

/// Calculate baseline parallelism for the given environment
fn calculate_baseline_parallelism(env: RuntimeEnv) -> Result<u32, DbInfraError> {
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

/// Build connection settings from environment and pool purpose
pub fn build_connection_settings(
    env: RuntimeEnv,
    purpose: PoolPurpose,
) -> Result<ConnectionSettings, DbInfraError> {
    let baseline = calculate_baseline_parallelism(env)?;

    let (pool_min, pool_max, acquire_timeout_ms, db_settings) = match env {
        RuntimeEnv::Prod => {
            let pool_max = (baseline + 2).clamp(8, 16);
            let db_settings = DbSettings {
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
        RuntimeEnv::Test => {
            let pool_max = (baseline + 2).clamp(8, 16);
            let db_settings = DbSettings {
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

/// Build ordered session-level SQL statements for Postgres
pub fn build_session_statements(settings: &DbSettings) -> Vec<String> {
    let mut stmts = vec![
        format!("SET application_name = '{}';", settings.app_name.replace('\'', "''")),
        "SET timezone = 'UTC';".to_string(),
        format!("SET statement_timeout = '{}';", settings.statement_timeout),
        format!(
            "SET idle_in_transaction_session_timeout = '{}';",
            settings.idle_in_transaction_timeout
        ),
    ];
    if let Some(lock_timeout) = &settings.lock_timeout {
        stmts.push(format!("SET lock_timeout = '{}';", lock_timeout));
    }
    stmts
}

/// Sanitize database URL by masking password in connection strings.
pub fn sanitize_db_url(url: &str) -> String {
    if url.contains('@') && url.contains(':') {
        let parts: Vec<&str> = url.split('@').collect();
        if parts.len() == 2 {
            let auth_part = parts[0];
            let host_part = parts[1];

            if let Some(colon_pos) = auth_part.rfind(':') {
                let scheme_user = &auth_part[..colon_pos];
                return format!("{}:***@{}", scheme_user, host_part);
            }
        }
    }
    url.to_string()
}

/// Redact password from connection string for safe logging.
fn redact_conn_spec_for_log(url: &str) -> String {
    if url.starts_with("postgresql://") {
        if let Some(auth_end) = url.find('@') {
            let scheme_end = url.find("://").unwrap_or(0) + 3;
            let creds = &url[scheme_end..auth_end];
            if let Some(colon) = creds.find(':') {
                let user = &creds[..colon];
                return format!("postgresql://{user}:***@{rest}", rest = &url[auth_end + 1..]);
            }
        }
    }
    url.to_string()
}

/// Create connection string from environment and database owner
pub fn make_conn_spec(env: RuntimeEnv, owner: DbOwner) -> Result<String, DbInfraError> {
    let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());

    let db_name = match env {
        RuntimeEnv::Prod => must_var("PROD_DB")?,
        RuntimeEnv::Test => {
            let db_name = must_var("TEST_DB")?;
            validate_test_db_name(&db_name)?;
            db_name
        }
    };

    let (username_raw, password_raw) = match owner {
        DbOwner::App => (must_var("APP_DB_USER")?, must_var("APP_DB_PASSWORD")?),
        DbOwner::Owner => (
            must_var("NOMMIE_OWNER_USER")?,
            must_var("NOMMIE_OWNER_PASSWORD")?,
        ),
    };

    let username = encode_userinfo(&username_raw);
    let password = encode_userinfo(&password_raw);

    let mut url = format!("postgresql://{username}:{password}@{host}:{port}/{db_name}");

    let ssl_mode =
        env::var("POSTGRES_SSL_MODE").unwrap_or_else(|_| "verify-full".to_string());

    if !ssl_mode.eq_ignore_ascii_case("disable") {
        let root_cert = must_var("POSTGRES_SSL_ROOT_CERT")?;

        let separator = if url.contains('?') { '&' } else { '?' };
        url.push(separator);
        url.push_str(&format!("sslmode={}", ssl_mode));
        url.push_str("&sslrootcert=");
        url.push_str(&root_cert);
    }

    trace!(conn_spec = %redact_conn_spec_for_log(&url), "make_conn_spec returning postgres connection string");
    Ok(url)
}

/// Database owner enum for different access levels
#[derive(Debug, Clone, PartialEq)]
pub enum DbOwner {
    /// Application-level access (limited permissions)
    App,
    /// Owner-level access (full permissions for migrations)
    Owner,
}

/// Validate that a test database name ends with `_test` to prevent accidental production use.
pub fn validate_test_db_name(db_name: &str) -> Result<(), DbInfraError> {
    if db_name.ends_with("_test") {
        Ok(())
    } else {
        Err(DbInfraError::Config {
            message: "test database name must end with _test".to_string(),
        })
    }
}

/// Get required environment variable or return error
fn must_var(name: &str) -> Result<String, DbInfraError> {
    env::var(name).map_err(|e| DbInfraError::Config {
        message: format!("environment variable missing: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_test_db_name_accepts_suffix() {
        assert!(validate_test_db_name("nommie_test").is_ok());
        assert!(validate_test_db_name("my_app_test").is_ok());
    }

    #[test]
    fn test_validate_test_db_name_rejects_missing_suffix() {
        let err = validate_test_db_name("nommie_prod").unwrap_err();
        assert!(matches!(err, DbInfraError::Config { .. }));
        assert!(err.to_string().contains("_test"));
    }
}
