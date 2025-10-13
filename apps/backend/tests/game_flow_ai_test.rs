mod common;
mod support;

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
/// This test:
/// 1. Creates a game with 4 AI players (seeded for determinism)
/// 2. Marks all AI players ready (triggers auto-start and AI orchestration)
/// 3. Verifies the game progresses through at least one complete round
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

    // Create 4 AI players with deterministic seeds and full memory
    let ai_service = AiService::new();
    let ai1 = ai_service
        .create_ai_user(txn, "random", Some(json!({"seed": 12345})), Some(100))
        .await?;
    let ai2 = ai_service
        .create_ai_user(txn, "random", Some(json!({"seed": 67890})), Some(100))
        .await?;
    let ai3 = ai_service
        .create_ai_user(txn, "random", Some(json!({"seed": 11111})), Some(100))
        .await?;
    let ai4 = ai_service
        .create_ai_user(txn, "random", Some(json!({"seed": 22222})), Some(100))
        .await?;

    let game_id = support::factory::create_fresh_lobby_game(txn).await?;

    // Add AI players to game as memberships
    use backend::repos::memberships;
    memberships::create_membership(txn, game_id, ai1, 0, false, memberships::GameRole::Player)
        .await?;
    memberships::create_membership(txn, game_id, ai2, 1, false, memberships::GameRole::Player)
        .await?;
    memberships::create_membership(txn, game_id, ai3, 2, false, memberships::GameRole::Player)
        .await?;
    memberships::create_membership(txn, game_id, ai4, 3, false, memberships::GameRole::Player)
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

    println!(
        "✅ Game completed successfully: {} rounds, state: {:?}",
        game.current_round.unwrap(),
        game.state
    );

    // Rollback transaction
    shared.rollback().await?;

    Ok(())
}
