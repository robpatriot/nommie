use crate::{
    error::AppError,
    infra::db::connect_db,
    state::{AppState, SecurityConfig},
};

/// Builder for creating AppState instances (used in both tests and main)
pub struct StateBuilder {
    security_config: SecurityConfig,
    db_profile: crate::config::db::DbProfile,
    owner: crate::config::db::DbOwner,
}

impl StateBuilder {
    /// Create a new StateBuilder with default settings
    pub fn new() -> Self {
        Self {
            security_config: SecurityConfig::default(),
            db_profile: crate::config::db::DbProfile::Prod,
            owner: crate::config::db::DbOwner::App,
        }
    }

    /// Set the database profile
    pub fn with_db(mut self, profile: crate::config::db::DbProfile) -> Self {
        self.db_profile = profile;
        self
    }

    /// Set the database owner
    pub fn with_owner(mut self, owner: crate::config::db::DbOwner) -> Self {
        self.owner = owner;
        self
    }

    /// Override the security configuration
    pub fn with_security(mut self, security_config: SecurityConfig) -> Self {
        self.security_config = security_config;
        self
    }

    /// Build the AppState
    pub async fn build(self) -> Result<AppState, AppError> {
        let db = connect_db(self.db_profile, self.owner).await?;
        Ok(AppState::new(db, self.security_config))
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
/// use backend::config::db::{DbProfile, DbOwner};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let state = build_state().with_db(DbProfile::Test).build().await?;
/// # Ok(())
/// # }
/// ```
pub fn build_state() -> StateBuilder {
    StateBuilder::new()
}
