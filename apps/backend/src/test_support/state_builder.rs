//! Test AppState builder (two-stage test harness, stage 1).

use sea_orm::DatabaseConnection;

use crate::error::AppError;
use crate::state::app_state::AppState;
use crate::state::security_config::SecurityConfig;

/// Entry-point: create a test-state builder.
pub fn create_test_state() -> TestStateBuilder {
    TestStateBuilder {
        want_db: false,
        security: None,
    }
}

#[derive(Debug, Clone)]
pub struct TestStateBuilder {
    want_db: bool,
    security: Option<SecurityConfig>,
}

impl TestStateBuilder {
    /// Enable a test database (connect/prepare). Uses your project's normal DB bootstrap.
    pub fn with_db(mut self) -> Self {
        self.want_db = true;
        self
    }

    /// Override the default security config if a test needs a special one.
    pub fn with_security(mut self, security: SecurityConfig) -> Self {
        self.security = Some(security);
        self
    }

    /// Build an `AppState`. `AppState` must be `Clone` in your project.
    pub async fn build(self) -> Result<AppState, AppError> {
        let security = self.security.unwrap_or_default();

        // Connect DB only if requested.
        let db: Option<DatabaseConnection> = if self.want_db {
            Some(connect_test_db().await?)
        } else {
            None
        };

        let state = AppState {
            db,
            security_config: security,
        };

        Ok(state)
    }
}

/// Connect to the test database for SeaORM.
/// Uses the project's canonical test DB bootstrap (env guard, migrations, etc.)
async fn connect_test_db() -> Result<DatabaseConnection, AppError> {
    use crate::test_support::{get_test_db_url, schema_guard::ensure_schema_ready};

    let url = get_test_db_url();
    let db = sea_orm::Database::connect(&url)
        .await
        .map_err(AppError::from)?;

    // Ensure schema is ready (this will panic if not)
    ensure_schema_ready(&db).await;

    Ok(db)
}
