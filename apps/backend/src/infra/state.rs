use crate::config::db::{DbOwner, DbProfile};
use crate::error::AppError;
use crate::infra::db::bootstrap_db;
use crate::state::app_state::AppState;
use crate::state::security_config::SecurityConfig;

/// Builder for creating AppState instances (used in both tests and main)
pub struct StateBuilder {
    security_config: SecurityConfig,
    db_profile: Option<DbProfile>,
}

impl StateBuilder {
    pub fn new() -> Self {
        Self {
            security_config: SecurityConfig::default(),
            db_profile: None,
        }
    }
    pub fn with_db(mut self, profile: DbProfile) -> Self {
        self.db_profile = Some(profile);
        self
    }
    pub fn with_security(mut self, security_config: SecurityConfig) -> Self {
        self.security_config = security_config;
        self
    }

    pub async fn build(self) -> Result<AppState, AppError> {
        if let Some(profile) = self.db_profile {
            // single entrypoint: build + migrate
            let conn = bootstrap_db(profile, DbOwner::App).await?;
            Ok(AppState::new(conn, self.security_config))
        } else {
            Ok(AppState::new_without_db(self.security_config))
        }
    }
}

impl Default for StateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
pub fn build_state() -> StateBuilder {
    StateBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_build_succeeds_without_db_option() {
        let state = build_state().build().await.unwrap();
        assert!(state.db().is_none());
    }
}
