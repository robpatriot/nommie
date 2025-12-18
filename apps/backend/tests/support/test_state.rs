use std::env;
use std::env::VarError;
use std::str::FromStr;

use backend::config::db::{DbKind, RuntimeEnv};
use backend::infra::state::{build_state, StateBuilder};
use backend::state::app_state::AppState;
use backend::AppError;

fn read_env_db_kind() -> Result<Option<String>, AppError> {
    match env::var("NOMMIE_TEST_DB_KIND") {
        Ok(value) => Ok(Some(value)),
        Err(VarError::NotPresent) => Ok(None),
        Err(err) => Err(AppError::config("failed to read NOMMIE_TEST_DB_KIND", err)),
    }
}

pub fn resolve_test_db_kind() -> Result<DbKind, AppError> {
    let maybe_value = read_env_db_kind()?;
    let resolved = match maybe_value {
        Some(ref raw) => DbKind::from_str(raw.as_str())?,
        None => DbKind::Postgres,
    };
    Ok(resolved)
}

pub fn test_state_builder() -> Result<StateBuilder, AppError> {
    let db_kind = resolve_test_db_kind()?;
    Ok(build_state().with_env(RuntimeEnv::Test).with_db(db_kind))
}

pub async fn build_test_state() -> Result<AppState, AppError> {
    test_state_builder()?.build().await
}
