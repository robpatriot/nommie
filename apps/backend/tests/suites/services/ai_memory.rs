// Tests for AI-specific memory modes and card play access.

use backend::ai::memory::{get_round_card_plays, MemoryMode};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::AppError;
use uuid::Uuid;

use crate::support::ai_memory_helpers::memory_mode_to_db_value;
use crate::support::build_test_state;
use crate::support::test_utils::test_seed;

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

    // Test to_db_value conversion
    assert_eq!(memory_mode_to_db_value(MemoryMode::Full), Some(100));
    assert_eq!(memory_mode_to_db_value(MemoryMode::None), Some(0));
    assert_eq!(
        memory_mode_to_db_value(MemoryMode::Partial { level: 75 }),
        Some(75)
    );
}

#[actix_web::test]
async fn test_get_round_card_plays_empty_round() -> Result<(), AppError> {
    let state = build_test_state().await?;
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
        rules_version: Set("1".to_string()),
        rng_seed: Set(Some(test_seed("round_card_plays_empty"))),
        current_round: Set(Some(1i16)),
        starting_dealer_pos: Set(Some(0i16)),
        current_trick_no: Set(0i16),
        current_round_id: Set(None),
        version: Set(0),
    }
    .insert(txn)
    .await?;

    let round = game_rounds::ActiveModel {
        id: NotSet,
        game_id: Set(game.id),
        round_no: Set(1i16),
        trump: Set(None),
        created_at: Set(OffsetDateTime::now_utc()),
        completed_at: Set(None),
    }
    .insert(txn)
    .await?;

    // Test with Full mode - should return empty vec (no plays yet)
    let plays = get_round_card_plays(txn, round.id, MemoryMode::Full, None, false).await?;
    assert!(plays.is_empty());

    // Test with None mode - should return empty vec
    let plays = get_round_card_plays(txn, round.id, MemoryMode::None, None, false).await?;
    assert!(plays.is_empty());

    // Test with Partial mode - should return empty vec
    let plays = get_round_card_plays(
        txn,
        round.id,
        MemoryMode::Partial { level: 50 },
        Some(42),
        false,
    )
    .await?;
    assert!(plays.is_empty());

    // Rollback the transaction immediately after last DB access
    shared.rollback().await?;

    Ok(())
}

#[actix_web::test]
async fn test_get_round_card_plays_with_tricks() -> Result<(), AppError> {
    let state = build_test_state().await?;
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
        rules_version: Set("1".to_string()),
        rng_seed: Set(Some(test_seed("round_card_plays_tricks"))),
        current_round: Set(Some(1i16)),
        starting_dealer_pos: Set(Some(0i16)),
        current_trick_no: Set(3i16),
        current_round_id: Set(None),
        version: Set(0),
    }
    .insert(txn)
    .await?;

    let round = game_rounds::ActiveModel {
        id: NotSet,
        game_id: Set(game.id),
        round_no: Set(1i16),
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
        trick_no: Set(1i16),
        lead_suit: Set(CardSuit::Hearts),
        winner_seat: Set(2i16),
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
            player_seat: Set(seat as i16),
            card: Set(json!({"suit": suit, "rank": rank})),
            play_order: Set(seat as i16),
            played_at: Set(OffsetDateTime::now_utc()),
        }
        .insert(txn)
        .await?;
    }

    // Create trick 2 with 4 plays
    let trick1 = round_tricks::ActiveModel {
        id: NotSet,
        round_id: Set(round.id),
        trick_no: Set(2i16),
        lead_suit: Set(CardSuit::Spades),
        winner_seat: Set(1i16),
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
            player_seat: Set(seat as i16),
            card: Set(json!({"suit": suit, "rank": rank})),
            play_order: Set(((seat + 2) % 4) as i16),
            played_at: Set(OffsetDateTime::now_utc()),
        }
        .insert(txn)
        .await?;
    }

    // Test Full mode - should return all tricks with Exact memory
    let plays = get_round_card_plays(txn, round.id, MemoryMode::Full, None, false).await?;
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
    let plays = get_round_card_plays(txn, round.id, MemoryMode::None, None, false).await?;
    assert!(plays.is_empty());

    // Test Partial mode with seed - should return degraded memory
    let plays_partial = get_round_card_plays(
        txn,
        round.id,
        MemoryMode::Partial { level: 50 },
        Some(42),
        false,
    )
    .await?;

    // Rollback the transaction immediately after last DB access
    shared.rollback().await?;

    assert_eq!(plays_partial.len(), 2); // Same number of tricks

    // Note: At level 50 with only 8 cards (2 tricks * 4 cards), degradation is probabilistic.
    // We don't assert degradation here since it may not occur in small samples.

    Ok(())
}

#[actix_web::test]
async fn test_ai_profile_memory_level_persistence() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    use backend::ai::RandomPlayer;
    use backend::repos::ai_profiles;

    // Create AI profile variant with memory level 75
    let variant = format!("test-{}", Uuid::new_v4());
    let profile = ai_profiles::create_profile(
        txn,
        ai_profiles::AiProfileDraft::new(
            RandomPlayer::NAME,
            RandomPlayer::VERSION,
            &variant,
            "Test AI",
        )
        .with_playstyle("random")
        .with_difficulty(5)
        .with_memory_level(75),
    )
    .await?;

    assert_eq!(profile.display_name, "Test AI");
    assert_eq!(profile.memory_level, Some(75));

    // Load it back
    let loaded = ai_profiles::find_by_id(txn, profile.id)
        .await?
        .expect("Profile should exist");

    assert_eq!(loaded.display_name, "Test AI");
    assert_eq!(loaded.memory_level, Some(75));

    // Update memory level to 100 (Full)
    let mut updated_profile = loaded.clone();
    updated_profile.memory_level = Some(100);
    let updated = ai_profiles::update_profile(txn, updated_profile).await?;

    // Rollback the transaction immediately after last DB access
    shared.rollback().await?;

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
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    use backend::ai::memory::MemoryMode;
    use backend::ai::RandomPlayer;
    use backend::repos::ai_profiles;

    // Validate seeded catalog memory levels
    let random_profile = ai_profiles::find_by_registry_variant(
        txn,
        RandomPlayer::NAME,
        RandomPlayer::VERSION,
        "default",
    )
    .await?
    .expect("RandomPlayer profile missing");
    assert_eq!(random_profile.memory_level, Some(50));
    assert_eq!(
        MemoryMode::from_db_value(random_profile.memory_level),
        MemoryMode::Partial { level: 50 }
    );

    let heuristic_profile = ai_profiles::find_by_registry_variant(
        txn,
        backend::ai::Heuristic::NAME,
        backend::ai::Heuristic::VERSION,
        "default",
    )
    .await?
    .expect("Heuristic profile missing");
    assert_eq!(heuristic_profile.memory_level, Some(80));
    assert_eq!(
        MemoryMode::from_db_value(heuristic_profile.memory_level),
        MemoryMode::Partial { level: 80 }
    );

    shared.rollback().await?;

    Ok(())
}
