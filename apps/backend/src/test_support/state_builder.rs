use crate::{
    bootstrap::db::{connect_db, DbProfile},
    error::AppError,
    state::{AppState, SecurityConfig},
};

/// Builder for creating AppState instances (used in both tests and main)
pub struct StateBuilder {
    security_config: SecurityConfig,
    db_profile: Option<DbProfile>,
}

impl StateBuilder {
    /// Create a new StateBuilder with default settings
    pub fn new() -> Self {
        Self {
            security_config: SecurityConfig::default(),
            db_profile: None,
        }
    }

    /// Enable database connection with explicit profile
    pub fn with_db(mut self, profile: DbProfile) -> Self {
        self.db_profile = Some(profile);
        self
    }

    /// Override the security configuration
    pub fn with_security(mut self, security_config: SecurityConfig) -> Self {
        self.security_config = security_config;
        self
    }

    /// Build the AppState
    pub async fn build(self) -> Result<AppState, AppError> {
        if let Some(profile) = self.db_profile {
            let db = connect_db(profile).await?;
            Ok(AppState::new(db, self.security_config))
        } else {
            Ok(AppState::without_db(self.security_config))
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
/// use backend::test_support::build_state;
/// use backend::bootstrap::db::DbProfile;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let state = build_state().with_db(DbProfile::Test).build().await?;
/// # Ok(())
/// # }
/// ```
pub fn build_state() -> StateBuilder {
    StateBuilder::new()
}
