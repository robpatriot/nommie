// StateBuilder / AppState build integration tests
//
// Tests for the full StateBuilder pipeline: DB bootstrap, Redis config,
// readiness manager, and env validation.

use std::sync::Arc;

use backend::config::db::RuntimeEnv;
use backend::db::txn::with_txn;
use backend::infra::state::build_state;
use backend::readiness::ReadinessManager;
use backend::AppError;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

#[tokio::test]
async fn builds_without_db() -> Result<(), AppError> {
    let state = build_state().build().await?;
    assert!(state.db().is_none());
    Ok(())
}

#[tokio::test]
async fn builds_with_test_db() -> Result<(), AppError> {
    let state = build_state().with_env(RuntimeEnv::Test).build().await?;
    assert!(state.db().is_some());

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let stmt = Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT 1 as test_value".to_owned(),
            );
            let row = txn.query_one(stmt).await?.expect("should get a row");
            let value: i32 = row.try_get("", "test_value")?;
            assert_eq!(value, 1);
            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_env_without_redis_url_disables_redis_and_allows_readiness() -> Result<(), AppError> {
    let manager = Arc::new(ReadinessManager::new());
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_readiness(manager.clone())
        .build()
        .await?;

    assert!(state.config.redis_url.0.is_none());

    manager.set_migration_result(true, None);

    for _ in 0..2 {
        manager.update_dependency(
            backend::readiness::types::DependencyName::Postgres,
            backend::readiness::types::DependencyCheck::Ok {
                latency: std::time::Duration::from_millis(1),
            },
        );
    }

    assert!(manager.is_ready());

    let internal = manager.to_internal_json();
    let deps = internal["dependencies"].as_array().unwrap();
    let redis = deps
        .iter()
        .find(|d| d["name"] == "redis")
        .expect("redis dependency present");
    assert_eq!(redis["status"]["state"], "disabled");
    assert_eq!(
        redis["status"]["reason"],
        "redis_url not configured in test env"
    );

    Ok(())
}

#[tokio::test]
async fn non_test_env_without_redis_url_fails_fast() {
    let manager = Arc::new(ReadinessManager::new());
    let result = build_state()
        .with_env(RuntimeEnv::Prod)
        .with_readiness(manager)
        .build()
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code(), backend::errors::ErrorCode::ConfigError);
}

#[tokio::test]
async fn test_env_with_redis_url_keeps_redis_required() -> Result<(), AppError> {
    let manager = Arc::new(ReadinessManager::new());
    let _state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_redis_url(Some("redis://localhost:6379".to_string()))
        .with_readiness(manager.clone())
        .build()
        .await?;

    let redis_down = backend::readiness::types::DependencyCheck::Down {
        error: "connection refused".to_string(),
        latency: std::time::Duration::from_millis(1),
    };
    manager.update_dependency(
        backend::readiness::types::DependencyName::Redis,
        redis_down.clone(),
    );
    manager.update_dependency(backend::readiness::types::DependencyName::Redis, redis_down);

    assert!(
        !manager.is_ready(),
        "readiness must not become ready when Redis is configured but not healthy"
    );

    let internal = manager.to_internal_json();
    let deps = internal["dependencies"].as_array().unwrap();
    let redis = deps
        .iter()
        .find(|d| d["name"] == "redis")
        .expect("redis dependency present");
    assert_ne!(redis["status"]["state"], "disabled");

    Ok(())
}

#[tokio::test]
async fn redis_connect_failure_marks_dependency_down() -> Result<(), AppError> {
    let manager = Arc::new(ReadinessManager::new());

    let result = build_state()
        .with_env(RuntimeEnv::Test)
        .with_redis_url(Some("invalid-redis-url".to_string()))
        .with_readiness(manager.clone())
        .build()
        .await;

    assert!(result.is_ok());

    let internal = manager.to_internal_json();
    let deps = internal["dependencies"].as_array().unwrap();
    let redis = deps
        .iter()
        .find(|d| d["name"] == "redis")
        .expect("redis dependency present after failed connect");
    assert_eq!(redis["status"]["state"], "down");
    assert!(redis["consecutive_failures"].as_u64().unwrap_or(0) >= 1);

    Ok(())
}
