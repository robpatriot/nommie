// Tests for extractors with SharedTxn injection
//
// These tests verify that DB-reading extractors can optionally use
// injected SharedTxn when present, and fall back to pooled connections otherwise.

use actix_web::{test, web, FromRequest};
use backend::config::db::{DbKind, RuntimeEnv};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::extractors::current_user_db::CurrentUserRecord;
use backend::extractors::game_id::GameId;
use backend::infra::state::build_state;
use backend::state::security_config::SecurityConfig;
use backend::utils::unique::{unique_email, unique_str};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use time::OffsetDateTime;

use crate::support::auth::bearer_header;
use crate::support::factory::seed_user_with_sub;

/// Insert a minimal game, return its id
async fn insert_test_game(
    db: &(impl ConnectionTrait + Send),
) -> Result<i64, Box<dyn std::error::Error>> {
    let now = OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        rules_version: Set("1.0.0".to_string()),
        lock_version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let game = game.insert(db).await?;
    Ok(game.id)
}

#[actix_web::test]
async fn test_current_user_db_with_shared_txn() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with Test DB + Security (JWT)
    let security_config =
        SecurityConfig::new("test_secret_key_for_testing_purposes_only".as_bytes());
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .with_security(security_config.clone())
        .build()
        .await?;

    // Open a shared txn first
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Seed user using the shared transaction
    let test_sub = unique_str("test-sub-shared");
    let test_email = unique_email("test-shared");
    let user = seed_user_with_sub(shared.transaction(), &test_sub, Some(&test_email))
        .await
        .expect("should create user successfully");

    // Request with state + auth header; inject shared txn
    let mut req = test::TestRequest::default()
        .insert_header((
            "Authorization",
            bearer_header(&test_sub, &test_email, &security_config),
        ))
        .app_data(web::Data::new(state))
        .to_http_request();
    shared.inject(&mut req);

    // Extract
    let mut payload = actix_web::dev::Payload::None;
    let result = CurrentUserRecord::from_request(&req, &mut payload).await?;

    // Verify
    assert_eq!(result.id, user.id);
    assert_eq!(result.sub, test_sub);
    assert_eq!(result.email, None);

    // Roll back shared txn
    drop(req);
    shared.rollback().await?;
    Ok(())
}

#[actix_web::test]
async fn test_game_id_with_shared_txn() -> Result<(), Box<dyn std::error::Error>> {
    // Build state with Test DB (no security needed)
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await?;

    // Open a shared txn first
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Insert game using the shared transaction
    let game_id = insert_test_game(shared.transaction()).await?;

    // Build request with path param, inject shared txn
    let game_id_str = game_id.to_string();
    let game_id_static: &'static str = Box::leak(game_id_str.into_boxed_str());
    let mut req = test::TestRequest::get()
        .param("game_id", game_id_static)
        .app_data(web::Data::new(state))
        .to_http_request();
    shared.inject(&mut req);

    // Extract
    let mut payload = actix_web::dev::Payload::None;
    let result = GameId::from_request(&req, &mut payload).await?;

    // Verify
    assert_eq!(result.0, game_id);

    // Cleanup
    drop(req);
    shared.rollback().await?;
    Ok(())
}
