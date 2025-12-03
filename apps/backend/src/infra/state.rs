use crate::config::db::{DbKind, RuntimeEnv};
use crate::config::email_allowlist::EmailAllowlist;
use crate::error::AppError;
use crate::infra::db::bootstrap_db;
use crate::state::app_state::AppState;
use crate::state::security_config::SecurityConfig;

/// Builder for creating AppState instances (used in both tests and main)
#[derive(Default)]
pub struct StateBuilder {
    security_config: SecurityConfig,
    env: Option<RuntimeEnv>,
    db_kind: Option<DbKind>,
    email_allowlist: Option<EmailAllowlist>,
}

impl StateBuilder {
    pub fn with_env(mut self, env: RuntimeEnv) -> Self {
        self.env = Some(env);
        self
    }

    pub fn with_db(mut self, db_kind: DbKind) -> Self {
        self.db_kind = Some(db_kind);
        self
    }

    pub fn with_security(mut self, security_config: SecurityConfig) -> Self {
        self.security_config = security_config;
        self
    }

    /// Set the email allowlist (None = disabled, Some = enabled)
    /// If not called, allowlist defaults to None (disabled)
    pub fn with_email_allowlist(mut self, allowlist: Option<EmailAllowlist>) -> Self {
        self.email_allowlist = allowlist;
        self
    }

    pub async fn build(self) -> Result<AppState, AppError> {
        match (self.env, self.db_kind) {
            (Some(env), Some(db_kind)) => {
                // Bootstrap database directly with env and db_kind
                let conn = bootstrap_db(env, db_kind).await?;
                Ok(AppState::new(
                    conn,
                    self.security_config,
                    self.email_allowlist,
                ))
            }
            _ => Ok(AppState::new_without_db(
                self.security_config,
                self.email_allowlist,
            )),
        }
    }
}

pub fn build_state() -> StateBuilder {
    StateBuilder::default()
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
