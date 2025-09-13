use crate::config::db::DbOwner;
use crate::error::AppError;
use crate::infra::db::connect_db;
use crate::infra::schema_guard::ensure_schema_ready;
use crate::state::app_state::AppState;
use crate::state::security_config::SecurityConfig;

/// Builder for creating AppState instances (used in both tests and main)
pub struct StateBuilder {
    security_config: SecurityConfig,
    db_profile: Option<crate::config::db::DbProfile>,
    existing_connection: Option<sea_orm::DatabaseConnection>,
}

impl StateBuilder {
    /// Create a new StateBuilder with default settings
    pub fn new() -> Self {
        Self {
            security_config: SecurityConfig::default(),
            db_profile: None,
            existing_connection: None,
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
    pub fn with_existing_db(mut self, connection: sea_orm::DatabaseConnection) -> Self {
        self.existing_connection = Some(connection);
        self
    }

    /// Build the AppState
    pub async fn build(self) -> Result<AppState, AppError> {
        let conn = if let Some(existing_conn) = self.existing_connection {
            // Use existing connection (for injection seam)
            existing_conn
        } else if let Some(profile) = self.db_profile {
            // Connect to real database and ensure schema is ready
            let conn = connect_db(profile, DbOwner::App).await?;
            ensure_schema_ready(&conn).await;
            conn
        } else {
            // No DB profile provided - panic with exact string
            panic!("AppState builder requires either a database profile or mock database.");
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
        expected = "AppState builder requires either a database profile or mock database."
    )]
    async fn test_build_panics_without_db_option() {
        // This should panic because no DB option is provided
        build_state().build().await.unwrap();
    }
}
