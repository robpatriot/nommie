use std::env;

use crate::error::AppError;

/// Database profile enum for different environments
#[derive(Debug, Clone, PartialEq)]
pub enum DbProfile {
    /// Production database profile
    Prod,
    /// Test database profile - enforces safety rules
    Test,
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

    let url = format!("postgresql://{username}:{password}@{host}:{port}/{db_name}");
    Ok(url)
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
        DbProfile::Prod => {
            let db_name = must_var("PROD_DB")?;
            Ok(db_name)
        }
        DbProfile::Test => {
            let db_name = must_var("TEST_DB")?;
            // Enforce safety: test DB must end with "_test"
            if !db_name.ends_with("_test") {
                return Err(AppError::config(format!(
                    "Test profile requires database name to end with '_test', but got: '{db_name}'"
                )));
            }
            Ok(db_name)
        }
    }
}

/// Get database credentials based on owner
fn credentials(owner: DbOwner) -> Result<(String, String), AppError> {
    match owner {
        DbOwner::App => {
            let username = must_var("APP_DB_USER")?;
            let password = must_var("APP_DB_PASSWORD")?;
            Ok((username, password))
        }
        DbOwner::Owner => {
            let username = must_var("NOMMIE_OWNER_USER")?;
            let password = must_var("NOMMIE_OWNER_PASSWORD")?;
            Ok((username, password))
        }
    }
}

/// Get required environment variable or return error
fn must_var(name: &str) -> Result<String, AppError> {
    env::var(name)
        .map_err(|_| AppError::config(format!("Required environment variable '{name}' is not set")))
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::{db_url, DbOwner, DbProfile};

    fn set_test_env() {
        env::set_var("PROD_DB", "nommie");
        env::set_var("TEST_DB", "nommie_test");
        env::set_var("APP_DB_USER", "nommie_app");
        env::set_var("APP_DB_PASSWORD", "app_password");
        env::set_var("NOMMIE_OWNER_USER", "nommie_owner");
        env::set_var("NOMMIE_OWNER_PASSWORD", "owner_password");
    }

    fn clear_test_env() {
        env::remove_var("PROD_DB");
        env::remove_var("TEST_DB");
        env::remove_var("APP_DB_USER");
        env::remove_var("APP_DB_PASSWORD");
        env::remove_var("NOMMIE_OWNER_USER");
        env::remove_var("NOMMIE_OWNER_PASSWORD");
        env::remove_var("POSTGRES_HOST");
        env::remove_var("POSTGRES_PORT");
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
        env::set_var("TEST_DB", "nommie_prod"); // Invalid: doesn't end with _test

        let result = db_url(DbProfile::Test, DbOwner::App);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("_test"));

        env::remove_var("TEST_DB");
        clear_test_env();
    }

    #[test]
    fn test_db_url_missing_env_var() {
        set_test_env();
        env::remove_var("PROD_DB");

        let result = db_url(DbProfile::Prod, DbOwner::App);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("PROD_DB"));

        clear_test_env();
    }
}
