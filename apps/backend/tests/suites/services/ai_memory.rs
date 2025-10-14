//! Tests for AI-specific memory modes and card play access.

use backend::ai::memory::{get_round_card_plays, MemoryMode};
use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::SharedTxn;
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
        created_by: Set(None),
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
    let plays = get_round_card_plays(txn, round.id, MemoryMode::Full, None).await?;
    assert!(plays.is_empty());

    // Test with None mode - should return empty vec
    let plays = get_round_card_plays(txn, round.id, MemoryMode::None, None).await?;
    assert!(plays.is_empty());

    // Test with Partial mode - should return empty vec
    let plays =
        get_round_card_plays(txn, round.id, MemoryMode::Partial { level: 50 }, Some(42)).await?;
    assert!(plays.is_empty());

    Ok(())
}

#[actix_web::test]
async fn test_get_round_card_plays_with_tricks() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("Failed to build test state");
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    use backend::entities::games::{self, GameState, GameVisibility};
    use backend::entities::round_tricks::{self, CardSuit};
    use backend::entities::{game_rounds, trick_plays};
    use sea_orm::{ActiveModelTrait, NotSet, Set};
    use serde_json::json;
    use time::OffsetDateTime;

    let game = games::ActiveModel {
        id: NotSet,
        created_by: Set(None),
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::TrickPlay),
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
        current_trick_no: Set(3),
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

    // Create trick 1 with 4 plays
    let trick0 = round_tricks::ActiveModel {
        id: NotSet,
        round_id: Set(round.id),
        trick_no: Set(1),
        lead_suit: Set(CardSuit::Hearts),
        winner_seat: Set(2),
        created_at: Set(OffsetDateTime::now_utc()),
    }
    .insert(txn)
    .await?;

    for (seat, suit, rank) in [
        (0, "HEARTS", "ACE"),
        (1, "HEARTS", "KING"),
        (2, "HEARTS", "QUEEN"),
        (3, "HEARTS", "JACK"),
    ] {
        trick_plays::ActiveModel {
            id: NotSet,
            trick_id: Set(trick0.id),
            player_seat: Set(seat),
            card: Set(json!({"suit": suit, "rank": rank})),
            play_order: Set(seat),
            played_at: Set(OffsetDateTime::now_utc()),
        }
        .insert(txn)
        .await?;
    }

    // Create trick 2 with 4 plays
    let trick1 = round_tricks::ActiveModel {
        id: NotSet,
        round_id: Set(round.id),
        trick_no: Set(2),
        lead_suit: Set(CardSuit::Spades),
        winner_seat: Set(1),
        created_at: Set(OffsetDateTime::now_utc()),
    }
    .insert(txn)
    .await?;

    for (seat, suit, rank) in [
        (2, "SPADES", "TEN"),
        (3, "SPADES", "NINE"),
        (0, "SPADES", "EIGHT"),
        (1, "SPADES", "SEVEN"),
    ] {
        trick_plays::ActiveModel {
            id: NotSet,
            trick_id: Set(trick1.id),
            player_seat: Set(seat),
            card: Set(json!({"suit": suit, "rank": rank})),
            play_order: Set((seat + 2) % 4),
            played_at: Set(OffsetDateTime::now_utc()),
        }
        .insert(txn)
        .await?;
    }

    // Test Full mode - should return all tricks with Exact memory
    let plays = get_round_card_plays(txn, round.id, MemoryMode::Full, None).await?;
    assert_eq!(plays.len(), 2);
    assert_eq!(plays[0].trick_no, 1);
    assert_eq!(plays[0].plays.len(), 4);
    assert_eq!(plays[1].trick_no, 2);
    assert_eq!(plays[1].plays.len(), 4);

    // Verify Full mode returns Exact memory
    for (_seat, play_memory) in &plays[0].plays {
        assert!(play_memory.is_exact(), "Full mode should have exact memory");
    }

    // Test None mode - should return empty
    let plays = get_round_card_plays(txn, round.id, MemoryMode::None, None).await?;
    assert!(plays.is_empty());

    // Test Partial mode with seed - should return degraded memory
    let plays_partial =
        get_round_card_plays(txn, round.id, MemoryMode::Partial { level: 50 }, Some(42)).await?;
    assert_eq!(plays_partial.len(), 2); // Same number of tricks

    // Note: At level 50 with only 8 cards (2 tricks * 4 cards), degradation is probabilistic.
    // We don't assert degradation here since it may not occur in small samples.
    // More comprehensive degradation tests are in ai_memory_degradation_test.rs

    Ok(())
}

#[actix_web::test]
async fn test_ai_profile_memory_level_persistence() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("Failed to build test state");
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    use backend::repos::{ai_profiles, users as users_repo};

    // Create a user
    let user = users_repo::create_user(txn, "ai_test_123", "Test AI", true).await?;

    // Create AI profile with memory level 75
    let profile = ai_profiles::create_profile(
        txn,
        user.id,
        Some("random".to_string()),
        Some(5),
        None,
        Some(75),
    )
    .await?;

    assert_eq!(profile.memory_level, Some(75));

    // Load it back
    let loaded = ai_profiles::find_by_user_id(txn, user.id)
        .await?
        .expect("Profile should exist");

    assert_eq!(loaded.memory_level, Some(75));

    // Update memory level to 100 (Full)
    let mut updated_profile = loaded.clone();
    updated_profile.memory_level = Some(100);
    let updated = ai_profiles::update_profile(txn, updated_profile).await?;

    assert_eq!(updated.memory_level, Some(100));

    // Verify MemoryMode conversions work with persisted values
    use backend::ai::memory::MemoryMode;
    assert_eq!(
        MemoryMode::from_db_value(updated.memory_level),
        MemoryMode::Full
    );

    Ok(())
}

#[actix_web::test]
async fn test_ai_service_creates_profile_with_memory_level() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("Failed to build test state");
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    use backend::ai::memory::MemoryMode;
    use backend::repos::ai_profiles;
    use backend::services::ai::AiService;
    use serde_json::json;

    let ai_service = AiService::new();

    // Create AI template user with Partial memory (level 60)
    let user_id = ai_service
        .create_ai_template_user(
            txn,
            "Random Bot (Partial Memory)",
            "random",
            Some(json!({"seed": 12345})),
            Some(60),
        )
        .await?;

    // Load the profile
    let profile = ai_profiles::find_by_user_id(txn, user_id)
        .await?
        .expect("AI profile should exist");

    assert_eq!(profile.memory_level, Some(60));
    assert_eq!(
        MemoryMode::from_db_value(profile.memory_level),
        MemoryMode::Partial { level: 60 }
    );

    // Create AI template user with Full memory (None -> defaults to Full)
    let user_id2 = ai_service
        .create_ai_template_user(
            txn,
            "Random Bot (Full Memory)",
            "random",
            Some(json!({"seed": 67890})),
            None,
        )
        .await?;

    let profile2 = ai_profiles::find_by_user_id(txn, user_id2)
        .await?
        .expect("AI profile should exist");

    assert_eq!(profile2.memory_level, None);
    assert_eq!(
        MemoryMode::from_db_value(profile2.memory_level),
        MemoryMode::Full
    );

    Ok(())
}
