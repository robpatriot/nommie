use backend::config::db::RuntimeEnv;
use backend::infra::state::{build_state, StateBuilder};
use backend::state::app_state::AppState;
use backend::AppError;

pub fn test_state_builder() -> Result<StateBuilder, AppError> {
    let redis_url = std::env::var("REDIS_URL").ok();
    Ok(build_state().with_env(RuntimeEnv::Test).with_redis_url(redis_url))
}

pub async fn build_test_state() -> Result<AppState, AppError> {
    test_state_builder()?.build().await
}
