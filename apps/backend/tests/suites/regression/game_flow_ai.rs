use backend::config::db::{DbKind, RuntimeEnv};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::games::GameState;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::services::ai::AiService;
use backend::services::game_flow::GameFlowService;
use serde_json::json;
use tracing::info;

use crate::support::test_utils::test_seed;

/// Test that a full game with 4 AI players completes successfully.
///
/// This test demonstrates the NEW reusable AI pattern:
/// 1. Creates reusable AI template users (only once)
/// 2. Uses the same AI templates in a game with per-instance overrides
/// 3. Marks all AI players ready (triggers auto-start and AI orchestration)
/// 4. Verifies the game completes all 26 rounds
#[tokio::test]
#[cfg_attr(not(feature = "regression-tests"), ignore)]
async fn test_full_game_with_ai_players() -> Result<(), AppError> {
    // Build test state
    let state = build_state()
        .with_env(RuntimeEnv::Test)
        .with_db(DbKind::Postgres)
        .build()
        .await
        .expect("build test state with DB");

    // Open SharedTxn for this test
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    // Create reusable AI template users with test-specific seeds
    let ai_service = AiService;

    // Generate unique seeds for each AI
    let ai1_seed = test_seed("full_game_ai_ai1");
    let ai2_seed = test_seed("full_game_ai_ai2");
    let ai3_seed = test_seed("full_game_ai_ai3");
    let ai4_seed = test_seed("full_game_ai_ai4");

    // Create 4 distinct AI template users
    let ai1 = ai_service
        .create_ai_template_user(
            txn,
            "Random Bot 1",
            "random",
            Some(json!({"seed": ai1_seed})),
            Some(100),
        )
        .await?;

    let ai2 = ai_service
        .create_ai_template_user(
            txn,
            "Random Bot 2",
            "random",
            Some(json!({"seed": ai2_seed})),
            Some(100),
        )
        .await?;

    let ai3 = ai_service
        .create_ai_template_user(
            txn,
            "Random Bot 3",
            "random",
            Some(json!({"seed": ai3_seed})),
            Some(100),
        )
        .await?;

    let ai4 = ai_service
        .create_ai_template_user(
            txn,
            "Random Bot 4",
            "random",
            Some(json!({"seed": ai4_seed})),
            Some(100),
        )
        .await?;

    let game_id = crate::support::factory::create_fresh_lobby_game(txn, "full_game_ai").await?;

    // Add AIs to game
    use backend::services::ai::AiInstanceOverrides;

    // Seat 0: Default config
    ai_service
        .add_ai_to_game(txn, game_id, ai1, 0, None)
        .await?;

    // Seat 1: Override name
    ai_service
        .add_ai_to_game(
            txn,
            game_id,
            ai2,
            1,
            Some(AiInstanceOverrides {
                name: Some("Custom Name Bot".to_string()),
                memory_level: None,
                config: None,
            }),
        )
        .await?;

    // Seat 2: Default config
    ai_service
        .add_ai_to_game(txn, game_id, ai3, 2, None)
        .await?;

    // Seat 3: Override config and memory
    let override_seed = test_seed("full_game_ai_ai4_override");
    ai_service
        .add_ai_to_game(
            txn,
            game_id,
            ai4,
            3,
            Some(AiInstanceOverrides {
                name: Some("Hard Mode Bot".to_string()),
                memory_level: Some(50),                       // Reduced memory
                config: Some(json!({"seed": override_seed})), // Test-specific seed
            }),
        )
        .await?;

    // Create gameflow service
    let service = GameFlowService;

    // Mark all AI players ready - this should trigger auto-start and AI orchestration
    service.mark_ready(txn, game_id, ai1).await?;
    service.mark_ready(txn, game_id, ai2).await?;
    service.mark_ready(txn, game_id, ai3).await?;
    service.mark_ready(txn, game_id, ai4).await?;

    // Load game and verify it completed
    let game = backend::adapters::games_sea::require_game(txn, game_id).await?;

    // Rollback transaction immediately after last DB access
    shared.rollback().await?;

    // Game should complete all 26 rounds
    assert_eq!(
        game.state,
        GameState::Completed,
        "Game should reach Completed state"
    );
    assert_eq!(
        game.current_round,
        Some(26),
        "Game should complete all 26 rounds"
    );

    info!(rounds = game.current_round.unwrap(), state = ?game.state, "Game completed successfully");

    Ok(())
}
