use backend::ai::RandomPlayer;
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::games::GameState;
use backend::error::AppError;
use backend::services::ai::{AiInstanceOverrides, AiService};
use serde_json::json;
use tracing::info;

use crate::support::build_test_state;
use crate::support::test_utils::test_seed;

/// Test that a full game with 4 AI players completes successfully.
///
/// This test demonstrates the NEW reusable AI pattern:
/// 1. Uses catalogued AI profiles for each seat
/// 2. Applies per-instance overrides (name/seed/memory) where needed
/// 3. AIs join the lobby already marked ready, triggering auto-start
/// 4. Verifies the game completes all 26 rounds
#[tokio::test]
#[ignore]
async fn test_full_game_with_ai_players() -> Result<(), AppError> {
    // Build test state
    let state = build_test_state().await?;

    // Open SharedTxn for this test
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    let ai_service = AiService;

    // Generate unique seeds for each AI
    let ai1_seed = test_seed("full_game_ai_ai1");
    let ai2_seed = test_seed("full_game_ai_ai2");
    let ai3_seed = test_seed("full_game_ai_ai3");
    let ai4_seed = test_seed("full_game_ai_ai4");

    let ai_profile = backend::repos::ai_profiles::find_by_registry_variant(
        txn,
        RandomPlayer::NAME,
        RandomPlayer::VERSION,
        "default",
    )
    .await?
    .expect("catalog profile missing");

    let game_id = crate::support::factory::create_fresh_lobby_game(txn, "full_game_ai").await?;

    // Add AIs to game
    // Seat 0: Default config
    ai_service
        .add_ai_to_game(
            txn,
            game_id,
            ai_profile.id,
            0,
            Some(AiInstanceOverrides {
                name: Some("Random Bot 1".into()),
                memory_level: None,
                config: Some(json!({"seed": ai1_seed})),
            }),
        )
        .await?;

    // Seat 1: Override name
    ai_service
        .add_ai_to_game(
            txn,
            game_id,
            ai_profile.id,
            1,
            Some(AiInstanceOverrides {
                name: Some("Custom Name Bot".to_string()),
                memory_level: None,
                config: Some(json!({"seed": ai2_seed})),
            }),
        )
        .await?;

    // Seat 2: Default config
    ai_service
        .add_ai_to_game(
            txn,
            game_id,
            ai_profile.id,
            2,
            Some(AiInstanceOverrides {
                name: Some("Random Bot 3".into()),
                memory_level: None,
                config: Some(json!({"seed": ai3_seed})),
            }),
        )
        .await?;

    // Seat 3: Override config and memory
    ai_service
        .add_ai_to_game(
            txn,
            game_id,
            ai_profile.id,
            3,
            Some(AiInstanceOverrides {
                name: Some("Hard Mode Bot".to_string()),
                memory_level: Some(50),                  // Reduced memory
                config: Some(json!({"seed": ai4_seed})), // Test-specific seed
            }),
        )
        .await?;

    // AI seats are auto-ready; no manual mark_ready needed

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
