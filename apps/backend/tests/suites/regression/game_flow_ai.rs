use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::games::GameState;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::services::ai::AiService;
use backend::services::game_flow::GameFlowService;
use serde_json::json;

/// Test that a full game with 4 AI players completes successfully.
///
/// This test demonstrates the NEW reusable AI pattern:
/// 1. Creates reusable AI template users (only once)
/// 2. Uses the same AI templates in a game with per-instance overrides
/// 3. Marks all AI players ready (triggers auto-start and AI orchestration)
/// 4. Verifies the game completes all 26 rounds
#[tokio::test]
#[cfg_attr(not(feature = "slow-tests"), ignore)]
async fn test_full_game_with_ai_players() -> Result<(), AppError> {
    // Build test state
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    // Open SharedTxn for this test
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    // Create reusable AI template users (these could be reused across many games)
    let ai_service = AiService::new();

    // Create 4 distinct AI template users to demonstrate reusable pattern
    let ai1 = ai_service
        .create_ai_template_user(
            txn,
            "Random Bot 1",
            "random",
            Some(json!({"seed": 12345})),
            Some(100),
        )
        .await?;

    let ai2 = ai_service
        .create_ai_template_user(
            txn,
            "Random Bot 2",
            "random",
            Some(json!({"seed": 67890})),
            Some(100),
        )
        .await?;

    let ai3 = ai_service
        .create_ai_template_user(
            txn,
            "Random Bot 3",
            "random",
            Some(json!({"seed": 11111})),
            Some(100),
        )
        .await?;

    let ai4 = ai_service
        .create_ai_template_user(
            txn,
            "Random Bot 4",
            "random",
            Some(json!({"seed": 22222})),
            Some(100),
        )
        .await?;

    let game_id = crate::support::factory::create_fresh_lobby_game(txn).await?;

    // Add AIs to game - demonstrating override capability
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

    // Seat 3: Override config and memory (demonstrates full override)
    ai_service
        .add_ai_to_game(
            txn,
            game_id,
            ai4,
            3,
            Some(AiInstanceOverrides {
                name: Some("Hard Mode Bot".to_string()),
                memory_level: Some(50),               // Reduced memory
                config: Some(json!({"seed": 99999})), // Different seed
            }),
        )
        .await?;

    // Create gameflow service
    let service = GameFlowService::new();

    // Mark all AI players ready - this should trigger auto-start and AI orchestration
    service.mark_ready(txn, game_id, ai1).await?;
    service.mark_ready(txn, game_id, ai2).await?;
    service.mark_ready(txn, game_id, ai3).await?;
    service.mark_ready(txn, game_id, ai4).await?;

    // Load game and verify it completed
    let game = backend::adapters::games_sea::require_game(txn, game_id).await?;

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

    tracing::info!(
        "✅ Game completed successfully: {} rounds, state: {:?}",
        game.current_round.unwrap(),
        game.state
    );
    tracing::info!("✅ Demonstrated reusable AI templates with per-instance overrides");

    // Rollback transaction
    shared.rollback().await?;

    Ok(())
}
