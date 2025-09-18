use crate::config::db::DbOwner;
use crate::error::AppError;
use crate::infra::db::connect_db;
use crate::infra::schema_guard::ensure_schema_ready;
use crate::state::app_state::AppState;
use crate::state::security_config::SecurityConfig;

/// Builder for creating AppState instances (used in both tests and main)
///
/// The StateBuilder supports one database connection path:
/// - `.with_db(..)` - Connect to a real database using a profile
///
/// By default, schema verification is performed unless `.assume_schema_ready()` is called.
/// If no database is configured, the AppState will be created without a database connection.
pub struct StateBuilder {
    security_config: SecurityConfig,
    db_profile: Option<crate::config::db::DbProfile>,
    assume_schema_ready: bool,
}

impl StateBuilder {
    /// Create a new StateBuilder with default settings
    pub fn new() -> Self {
        Self {
            security_config: SecurityConfig::default(),
            db_profile: None,
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

    /// Skip schema verification for the connection; advanced use.
    pub fn assume_schema_ready(mut self) -> Self {
        self.assume_schema_ready = true;
        self
    }

    /// Build the AppState
    pub async fn build(self) -> Result<AppState, AppError> {
        if let Some(profile) = self.db_profile {
            // Connect to real database and ensure schema is ready
            let conn = connect_db(profile, DbOwner::App).await?;
            if !self.assume_schema_ready {
                ensure_schema_ready(&conn).await;
            }
            Ok(AppState::new(conn, self.security_config))
        } else {
            // No DB profile provided - create AppState without database
            Ok(AppState::new_without_db(self.security_config))
        }
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
    async fn test_build_succeeds_without_db_option() {
        // This should succeed and create an AppState without a database
        let state = build_state().build().await.unwrap();
        assert!(state.db().is_none());
    }
}
