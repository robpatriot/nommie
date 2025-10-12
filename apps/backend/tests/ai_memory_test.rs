//! Tests for AI memory modes and game history access.

use backend::ai::memory::{get_round_card_plays, MemoryMode};
use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::domain::player_view::GameHistory;
use backend::error::AppError;
use backend::infra::state::build_state;

#[actix_web::test]
async fn test_memory_mode_conversions() {
    // Test MemoryMode conversions
    assert_eq!(MemoryMode::from_db_value(None), MemoryMode::Full);
    assert_eq!(MemoryMode::from_db_value(Some(100)), MemoryMode::Full);
    assert_eq!(MemoryMode::from_db_value(Some(0)), MemoryMode::None);
    assert_eq!(
        MemoryMode::from_db_value(Some(50)),
        MemoryMode::Partial { level: 50 }
    );

    // Test to_db_value
    assert_eq!(MemoryMode::Full.to_db_value(), Some(100));
    assert_eq!(MemoryMode::None.to_db_value(), Some(0));
    assert_eq!(MemoryMode::Partial { level: 75 }.to_db_value(), Some(75));
}

#[actix_web::test]
async fn test_get_round_card_plays_empty_round() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("Failed to build test state");
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    // Create a game and round but no card plays
    use backend::entities::game_rounds;
    use backend::entities::games::{self, GameState, GameVisibility};
    use sea_orm::{ActiveModelTrait, NotSet, Set};
    use time::OffsetDateTime;

    let game = games::ActiveModel {
        id: NotSet,
        created_by: Set(Some(1)),
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Bidding),
        created_at: Set(OffsetDateTime::now_utc()),
        updated_at: Set(OffsetDateTime::now_utc()),
        started_at: Set(None),
        ended_at: Set(None),
        name: Set(Some("Test Game".to_string())),
        join_code: Set(None),
        rules_version: Set("1".to_string()),
        rng_seed: Set(Some(12345)),
        current_round: Set(Some(1)),
        starting_dealer_pos: Set(Some(0)),
        current_trick_no: Set(0),
        current_round_id: Set(None),
        lock_version: Set(0),
    }
    .insert(txn)
    .await?;

    let round = game_rounds::ActiveModel {
        id: NotSet,
        game_id: Set(game.id),
        round_no: Set(1),
        hand_size: Set(13),
        dealer_pos: Set(0),
        trump: Set(None),
        created_at: Set(OffsetDateTime::now_utc()),
        completed_at: Set(None),
    }
    .insert(txn)
    .await?;

    // Test with Full mode - should return empty vec (no plays yet)
    let plays = get_round_card_plays(txn, round.id, MemoryMode::Full).await?;
    assert!(plays.is_empty());

    // Test with None mode - should return empty vec
    let plays = get_round_card_plays(txn, round.id, MemoryMode::None).await?;
    assert!(plays.is_empty());

    // Test with Partial mode - should return empty vec
    let plays = get_round_card_plays(txn, round.id, MemoryMode::Partial { level: 50 }).await?;
    assert!(plays.is_empty());

    Ok(())
}

#[actix_web::test]
async fn test_game_history_empty_game() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("Failed to build test state");
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    // Create a game with no rounds
    use backend::entities::games::{self, GameState, GameVisibility};
    use sea_orm::{ActiveModelTrait, NotSet, Set};
    use time::OffsetDateTime;

    let game = games::ActiveModel {
        id: NotSet,
        created_by: Set(Some(1)),
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        created_at: Set(OffsetDateTime::now_utc()),
        updated_at: Set(OffsetDateTime::now_utc()),
        started_at: Set(None),
        ended_at: Set(None),
        name: Set(Some("Test Game".to_string())),
        join_code: Set(None),
        rules_version: Set("1".to_string()),
        rng_seed: Set(Some(12345)),
        current_round: Set(None),
        starting_dealer_pos: Set(None),
        current_trick_no: Set(0),
        current_round_id: Set(None),
        lock_version: Set(0),
    }
    .insert(txn)
    .await?;

    // Load game history - should be empty
    let history = GameHistory::load(txn, game.id).await?;
    assert!(history.rounds.is_empty());

    Ok(())
}
