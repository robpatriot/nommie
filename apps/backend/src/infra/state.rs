use crate::config::db::DbOwner;
use crate::error::AppError;
use crate::infra::db::connect_db;
use crate::infra::schema_guard::ensure_schema_ready;
use crate::state::app_state::AppState;
use crate::state::security_config::SecurityConfig;

/// Builder for creating AppState instances (used in both tests and main)
///
/// The StateBuilder supports multiple database connection paths:
/// - `.with_db(..)` - Connect to a database using a profile
/// - `.sqlite_file(..)` - Override SQLite file for SqliteFile profile
pub struct StateBuilder {
    security_config: SecurityConfig,
    db_profile: Option<crate::config::db::DbProfile>,
}

impl StateBuilder {
    /// Create a new StateBuilder with default settings
    pub fn new() -> Self {
        Self {
            security_config: SecurityConfig::default(),
            db_profile: None,
        }
    }

    /// Set the database profile
    pub fn with_db(mut self, profile: crate::config::db::DbProfile) -> Self {
        self.db_profile = Some(profile);
        self
    }

    /// Override SQLite file for SqliteFile profile
    ///
    /// This method only has an effect when the current profile is `SqliteFile { file: None }`.
    /// It will update the embedded file option to the provided value.
    pub fn sqlite_file(mut self, file: impl Into<String>) -> Self {
        if let Some(crate::config::db::DbProfile::SqliteFile {
            file: ref mut file_option @ None,
        }) = self.db_profile
        {
            *file_option = Some(file.into());
        }
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
            // Connect to database
            let conn = connect_db(profile.clone(), DbOwner::App).await?;

            // Handle schema initialization based on profile type
            match profile {
                crate::config::db::DbProfile::InMemory => {
                    // InMemory databases are always fresh - skip schema check
                    // Tests will apply full migration manually using migrate_internal
                }
                _ => {
                    // PostgreSQL and SqliteFile profiles use standard schema check
                    // This ensures CLI migrations have been run first
                    ensure_schema_ready(&conn).await;
                }
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
