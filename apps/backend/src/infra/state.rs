use crate::config::db::DbOwner;
use crate::error::AppError;
use crate::infra::db::connect_db;
use crate::infra::schema_guard::ensure_schema_ready;
use crate::state::app_state::AppState;
use crate::state::security_config::SecurityConfig;

/// Error message for missing database configuration
pub const ERR_MISSING_DB: &str = "AppState builder requires a database: use with_db(..), with_existing_db(..), or with_mock_db().";

/// Builder for creating AppState instances (used in both tests and main)
///
/// The StateBuilder supports three database connection paths:
/// - `.with_db(..)` - Connect to a real database using a profile
/// - `.with_existing_db(..)` - Use an existing database connection
/// - `.with_mock_db()` - Use a mock database (test-only, via extension trait)
///
/// By default, schema verification is performed unless `.assume_schema_ready()` is called.
pub struct StateBuilder {
    security_config: SecurityConfig,
    db_profile: Option<crate::config::db::DbProfile>,
    existing_connection: Option<sea_orm::DatabaseConnection>,
    assume_schema_ready: bool,
}

impl StateBuilder {
    /// Create a new StateBuilder with default settings
    pub fn new() -> Self {
        Self {
            security_config: SecurityConfig::default(),
            db_profile: None,
            existing_connection: None,
            assume_schema_ready: false,
        }
    }

    /// Set the database profile
    pub fn with_db(mut self, profile: crate::config::db::DbProfile) -> Self {
        self.db_profile = Some(profile);
        self
    }

    /// Override the security configuration
    pub fn with_security(mut self, security_config: SecurityConfig) -> Self {
        self.security_config = security_config;
        self
    }

    /// Set an existing database connection (for injection seam)
    ///
    /// Verifies schema by default; call `.assume_schema_ready()` to skip (for
    /// mocks or advanced callers that guarantee migrations).
    pub fn with_existing_db(mut self, connection: sea_orm::DatabaseConnection) -> Self {
        self.existing_connection = Some(connection);
        self
    }

    /// Skip schema verification for the connection; advanced use.
    pub fn assume_schema_ready(mut self) -> Self {
        self.assume_schema_ready = true;
        self
    }

    /// Build the AppState
    pub async fn build(self) -> Result<AppState, AppError> {
        let conn = if let Some(existing_conn) = self.existing_connection {
            // Use existing connection (for injection seam)
            if !self.assume_schema_ready {
                ensure_schema_ready(&existing_conn).await;
            }
            existing_conn
        } else if let Some(profile) = self.db_profile {
            // Connect to real database and ensure schema is ready
            let conn = connect_db(profile, DbOwner::App).await?;
            if !self.assume_schema_ready {
                ensure_schema_ready(&conn).await;
            }
            conn
        } else {
            // No DB profile provided - panic with exact string
            panic!("{ERR_MISSING_DB}");
        };

        Ok(AppState::new(conn, self.security_config))
    }
}

impl Default for StateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new state builder
///
/// # Example
/// ```rust
/// use backend::infra::state::build_state;
/// use backend::config::db::DbProfile;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let state = build_state().with_db(DbProfile::Test).build().await?;
/// # Ok(())
/// # }
/// ```
pub fn build_state() -> StateBuilder {
    StateBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[should_panic(
        expected = "AppState builder requires a database: use with_db(..), with_existing_db(..), or with_mock_db()."
    )]
    async fn test_build_panics_without_db_option() {
        // This should panic because no DB option is provided
        build_state().build().await.unwrap();
    }
}
