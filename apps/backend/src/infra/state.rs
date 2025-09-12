#[cfg(any(test, feature = "mockstrict-default"))]
use {
    backend_test_support::mock_strict::register_mock_strict_connection,
    sea_orm::{DatabaseBackend, MockDatabase},
    std::sync::atomic::{AtomicUsize, Ordering},
    std::sync::Once,
};

use crate::config::db::DbOwner;
use crate::error::AppError;
use crate::infra::db::connect_db;
use crate::infra::schema_guard::ensure_schema_ready;
use crate::state::app_state::AppState;
use crate::state::security_config::SecurityConfig;

#[cfg(any(test, feature = "mockstrict-default"))]
static MOCKSTRICT_WARNING_ONCE: Once = Once::new();
#[cfg(any(test, feature = "mockstrict-default"))]
static MOCKSTRICT_WARNING_COUNT: AtomicUsize = AtomicUsize::new(0);

#[cfg(any(test, feature = "mockstrict-default"))]
fn emit_mockstrict_warning_once() {
    MOCKSTRICT_WARNING_ONCE.call_once(|| {
        log::warn!(
            "Using MockStrict DB for tests. Any DB access without a shared transaction will panic. Use .with_db(DbProfile::Test) or inject a shared transaction; see tests/support/shared_txn.rs."
        );
        MOCKSTRICT_WARNING_COUNT.fetch_add(1, Ordering::SeqCst);
    });
}

/// Determines whether to default to MockStrict when no DB profile is set
fn should_default_to_mockstrict() -> bool {
    cfg!(test) || cfg!(feature = "mockstrict-default")
}

/// Builder for creating AppState instances (used in both tests and main)
pub struct StateBuilder {
    security_config: SecurityConfig,
    db_profile: crate::config::db::DbProfile,
    db_profile_set: bool,
}

impl StateBuilder {
    /// Create a new StateBuilder with default settings
    pub fn new() -> Self {
        Self {
            security_config: SecurityConfig::default(),
            db_profile: crate::config::db::DbProfile::Prod,
            db_profile_set: false,
        }
    }

    /// Set the database profile
    pub fn with_db(mut self, profile: crate::config::db::DbProfile) -> Self {
        self.db_profile = profile;
        self.db_profile_set = true;
        self
    }

    /// Override the security configuration
    pub fn with_security(mut self, security_config: SecurityConfig) -> Self {
        self.security_config = security_config;
        self
    }

    /// Build the AppState
    pub async fn build(self) -> Result<AppState, AppError> {
        let conn = if !self.db_profile_set {
            if should_default_to_mockstrict() {
                // Use MockStrict fallback when appropriate
                #[cfg(any(test, feature = "mockstrict-default"))]
                {
                    let mock_db = MockDatabase::new(DatabaseBackend::Postgres);
                    let conn = mock_db.into_connection();
                    register_mock_strict_connection(&conn);
                    emit_mockstrict_warning_once();
                    conn
                }
                #[cfg(not(any(test, feature = "mockstrict-default")))]
                {
                    unreachable!()
                }
            } else {
                // Production panic when no DB profile is set
                panic!("AppState builder requires an explicit DB profile outside tests.");
            }
        } else if cfg!(test) && self.db_profile == crate::config::db::DbProfile::Test {
            // In tests with explicit Test profile, use real test DB
            connect_db(self.db_profile, DbOwner::App).await?
        } else {
            // Non-test or other profiles, use normal connection
            connect_db(self.db_profile, DbOwner::App).await?
        };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mockstrict_warning_emitted_once() {
        // Reset the counter for this test
        MOCKSTRICT_WARNING_COUNT.store(0, Ordering::SeqCst);

        // Call the helper function twice
        emit_mockstrict_warning_once();
        emit_mockstrict_warning_once();

        // Assert the counter is exactly 1 (warning emitted only once)
        assert_eq!(MOCKSTRICT_WARNING_COUNT.load(Ordering::SeqCst), 1);
    }
}
