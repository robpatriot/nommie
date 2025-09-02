use crate::{
    bootstrap::db::{connect_db, DbProfile},
    error::AppError,
    state::{AppState, SecurityConfig},
};

/// Builder for creating test AppState instances
pub struct TestStateBuilder {
    security_config: SecurityConfig,
    with_db: bool,
}

impl TestStateBuilder {
    /// Create a new TestStateBuilder with default settings
    pub fn new() -> Self {
        Self {
            security_config: SecurityConfig::default(),
            with_db: false,
        }
    }

    /// Enable database connection using connect_db(DbProfile::Test)
    pub fn with_db(mut self) -> Self {
        self.with_db = true;
        self
    }

    /// Override the security configuration
    pub fn with_security(mut self, security_config: SecurityConfig) -> Self {
        self.security_config = security_config;
        self
    }

    /// Build the AppState
    pub async fn build(self) -> Result<AppState, AppError> {
        if self.with_db {
            let db = connect_db(DbProfile::Test).await?;
            Ok(AppState::new(db, self.security_config))
        } else {
            Ok(AppState::without_db(self.security_config))
        }
    }
}

impl Default for TestStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new test state builder
///
/// # Example
/// ```rust
/// use backend::test_support::create_test_state;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let state = create_test_state().with_db().build().await?;
/// # Ok(())
/// # }
/// ```
pub fn create_test_state() -> TestStateBuilder {
    TestStateBuilder::new()
}
