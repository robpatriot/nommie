//! Tests for AI memory degradation functionality.

use backend::ai::memory::{get_round_card_plays, MemoryMode};
use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::error::AppError;
use backend::infra::state::build_state;

#[actix_web::test]
async fn test_memory_degradation_determinism() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("Failed to build test state");
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    // Create a game with some played cards
    let (round_id, _game_id) = create_test_round_with_plays(txn).await?;

    // Same seed should produce identical degradation
    let plays1 = get_round_card_plays(
        txn,
        round_id,
        MemoryMode::Partial { level: 50 },
        Some(12345),
    )
    .await?;
    let plays2 = get_round_card_plays(
        txn,
        round_id,
        MemoryMode::Partial { level: 50 },
        Some(12345),
    )
    .await?;

    assert_eq!(plays1.len(), plays2.len());
    for (trick1, trick2) in plays1.iter().zip(plays2.iter()) {
        assert_eq!(trick1.trick_no, trick2.trick_no);
        assert_eq!(trick1.plays.len(), trick2.plays.len());
        for ((seat1, mem1), (seat2, mem2)) in trick1.plays.iter().zip(trick2.plays.iter()) {
            assert_eq!(seat1, seat2);
            assert_eq!(
                mem1, mem2,
                "Same seed should produce identical memory degradation"
            );
        }
    }

    // Different seed should produce different degradation (with high probability)
    let plays3 = get_round_card_plays(
        txn,
        round_id,
        MemoryMode::Partial { level: 50 },
        Some(99999),
    )
    .await?;
    let mut found_difference = false;
    for (trick1, trick3) in plays1.iter().zip(plays3.iter()) {
        for ((_, mem1), (_, mem3)) in trick1.plays.iter().zip(trick3.plays.iter()) {
            if mem1 != mem3 {
                found_difference = true;
                break;
            }
        }
    }
    assert!(
        found_difference,
        "Different seeds should produce different degradation (probabilistic)"
    );

    Ok(())
}

#[actix_web::test]
async fn test_memory_levels() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("Failed to build test state");
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    let (round_id, _game_id) = create_test_round_with_plays(txn).await?;

    // Level 0 (None) - no memory
    let plays = get_round_card_plays(txn, round_id, MemoryMode::None, Some(42)).await?;
    assert!(plays.is_empty());

    // Level 100 (Full) - perfect memory
    let plays = get_round_card_plays(txn, round_id, MemoryMode::Full, Some(42)).await?;
    assert!(!plays.is_empty());
    for trick in &plays {
        for (_seat, play_memory) in &trick.plays {
            assert!(
                play_memory.is_exact(),
                "Level 100 should have perfect memory"
            );
        }
    }

    // Level 10 - very poor memory (should forget most)
    let plays =
        get_round_card_plays(txn, round_id, MemoryMode::Partial { level: 10 }, Some(42)).await?;
    let mut forgotten_count = 0;
    let mut total_count = 0;
    for trick in &plays {
        for (_seat, play_memory) in &trick.plays {
            total_count += 1;
            if play_memory.is_forgotten() {
                forgotten_count += 1;
            }
        }
    }
    assert!(
        forgotten_count > total_count / 2,
        "Level 10 should forget majority of cards"
    );

    Ok(())
}

#[actix_web::test]
async fn test_partial_memory_types() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("Failed to build test state");
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    let (round_id, _game_id) = create_test_round_with_plays(txn).await?;

    // Level 50 - moderate memory (should have mix of types)
    let plays =
        get_round_card_plays(txn, round_id, MemoryMode::Partial { level: 50 }, Some(42)).await?;

    use backend::domain::PlayMemory;

    let mut exact_count = 0;
    let mut suit_count = 0;
    let mut category_count = 0;
    let mut forgotten_count = 0;

    for trick in &plays {
        for (_seat, play_memory) in &trick.plays {
            match play_memory {
                PlayMemory::Exact(_) => exact_count += 1,
                PlayMemory::Suit(_) => suit_count += 1,
                PlayMemory::RankCategory(_) => category_count += 1,
                PlayMemory::Forgotten => forgotten_count += 1,
            }
        }
    }

    // At level 50, we should have some variety (probabilistic test)
    assert!(
        exact_count > 0,
        "Level 50 should remember some cards exactly"
    );
    // Should have at least one non-exact memory type
    assert!(
        suit_count + category_count + forgotten_count > 0,
        "Level 50 should have some degraded memory"
    );

    Ok(())
}

// Helper function to create a test round with some plays
async fn create_test_round_with_plays(
    txn: &sea_orm::DatabaseTransaction,
) -> Result<(i64, i64), AppError> {
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
        current_trick_no: Set(1),
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

    // Create one completed trick
    let trick = round_tricks::ActiveModel {
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
        (1, "HEARTS", "TWO"),
        (2, "HEARTS", "KING"),
        (3, "HEARTS", "THREE"),
    ] {
        trick_plays::ActiveModel {
            id: NotSet,
            trick_id: Set(trick.id),
            player_seat: Set(seat),
            card: Set(json!({"suit": suit, "rank": rank})),
            play_order: Set(seat),
            played_at: Set(OffsetDateTime::now_utc()),
        }
        .insert(txn)
        .await?;
    }

    Ok((round.id, game.id))
}
