use crate::config::db::DbOwner;
use crate::error::AppError;
use crate::infra::db::connect_db;
use crate::state::app_state::AppState;
use crate::state::security_config::SecurityConfig;
use crate::test_support::schema_guard::ensure_schema_ready;

/// Builder for creating AppState instances (used in both tests and main)
pub struct StateBuilder {
    security_config: SecurityConfig,
    db_profile: crate::config::db::DbProfile,
}

impl StateBuilder {
    /// Create a new StateBuilder with default settings
    pub fn new() -> Self {
        Self {
            security_config: SecurityConfig::default(),
            db_profile: crate::config::db::DbProfile::Prod,
        }
    }

    /// Set the database profile
    pub fn with_db(mut self, profile: crate::config::db::DbProfile) -> Self {
        self.db_profile = profile;
        self
    }

    /// Override the security configuration
    pub fn with_security(mut self, security_config: SecurityConfig) -> Self {
        self.security_config = security_config;
        self
    }

    /// Build the AppState
    pub async fn build(self) -> Result<AppState, AppError> {
        let conn = connect_db(self.db_profile, DbOwner::App).await?;

        // Ensure schema is ready (validates migrations table exists)
        ensure_schema_ready(&conn).await;

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
