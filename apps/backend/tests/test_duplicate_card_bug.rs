mod common;
mod support;

use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::domain::Card;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::services::ai::AiService;
use backend::services::game_flow::GameFlowService;
use serde_json::json;

/// Test that checks if duplicate cards can be played in different tricks.
///
/// This test is designed to expose a potential bug where:
/// 1. round_hands stores the original dealt hand (never updated)
/// 2. VisibleGameState.hand loads the full original hand
/// 3. legal_plays() doesn't filter out already-played cards
/// 4. An AI could theoretically play the same card multiple times
///
/// Expected behavior:
/// - The second play of the same card should FAIL with CardNotInHand error
///
/// Bug scenario:
/// - If validation is missing, the same card gets played in multiple tricks
#[tokio::test]
async fn test_cannot_play_same_card_twice() -> Result<(), AppError> {
    // Build test state
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    // Create 4 AI players with random AI (deterministic seed to potentially expose bug)
    let ai_service = AiService::new();
    let ai1 = ai_service
        .create_ai_user(txn, "random", Some(json!({"seed": 11111})))
        .await?;
    let ai2 = ai_service
        .create_ai_user(txn, "random", Some(json!({"seed": 22222})))
        .await?;
    let ai3 = ai_service
        .create_ai_user(txn, "random", Some(json!({"seed": 33333})))
        .await?;
    let ai4 = ai_service
        .create_ai_user(txn, "random", Some(json!({"seed": 44444})))
        .await?;

    // Create game
    use backend::entities::games::{self, GameState as DbGameState, GameVisibility};
    use sea_orm::{ActiveModelTrait, NotSet, Set};
    use time::OffsetDateTime;

    let creator_id = support::factory::create_test_user(txn, "creator", Some("Creator")).await?;
    let now = OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        id: NotSet,
        created_by: Set(Some(creator_id)),
        visibility: Set(GameVisibility::Public),
        state: Set(DbGameState::Lobby),
        created_at: Set(now),
        updated_at: Set(now),
        started_at: Set(None),
        ended_at: Set(None),
        name: Set(Some("Duplicate Card Test".to_string())),
        join_code: Set(Some(format!("TEST{}", rand::random::<u32>() % 1000000))),
        rules_version: Set("1.0".to_string()),
        rng_seed: Set(Some(99999)), // Fixed seed - should give 2 cards to start
        current_round: Set(None),
        starting_dealer_pos: Set(None),
        current_trick_no: Set(0),
        current_round_id: Set(None),
        lock_version: Set(0),
    };
    let game_id = game.insert(txn).await?.id;

    // Add AI players
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

    // Mark all ready to trigger auto-start
    service.mark_ready(txn, game_id, ai1).await?;
    service.mark_ready(txn, game_id, ai2).await?;
    service.mark_ready(txn, game_id, ai3).await?;
    service.mark_ready(txn, game_id, ai4).await?;

    // Game should start and play round 1 (2 cards per player)
    // If bug exists, we'll see cards played multiple times in the logs

    // Load all trick plays for round 1
    use backend::repos::{plays, rounds, tricks};
    let game = backend::adapters::games_sea::require_game(txn, game_id).await?;

    if let Some(round_no) = game.current_round {
        if let Some(round) = rounds::find_by_game_and_round(txn, game_id, round_no).await? {
            // Get all tricks for this round
            let all_tricks = tricks::find_all_by_round(txn, round.id).await?;

            println!("\n📊 ROUND {} ANALYSIS", round_no);
            println!("{}", "=".repeat(60));

            // For each player, collect all cards they played
            for seat in 0..4 {
                let mut played_cards: Vec<Card> = Vec::new();

                for trick in &all_tricks {
                    let trick_plays = plays::find_all_by_trick(txn, trick.id).await?;

                    for play in trick_plays {
                        if play.player_seat == seat {
                            let card = backend::domain::cards_parsing::from_stored_format(
                                &play.card.suit,
                                &play.card.rank,
                            )?;
                            played_cards.push(card);
                        }
                    }
                }

                println!(
                    "\nPlayer seat {}: Played {} cards",
                    seat,
                    played_cards.len()
                );
                for (i, card) in played_cards.iter().enumerate() {
                    println!("  Trick {}: {:?} of {:?}", i + 1, card.rank, card.suit);
                }

                // Check for duplicates
                let mut unique_cards = played_cards.clone();
                unique_cards.sort_by_key(|c| (c.suit as u8, c.rank as u8));
                unique_cards.dedup();

                if unique_cards.len() < played_cards.len() {
                    println!("  ⚠️  WARNING: Player {} played duplicate cards!", seat);
                    println!(
                        "     Total plays: {}, Unique cards: {}",
                        played_cards.len(),
                        unique_cards.len()
                    );

                    // Find which cards were duplicated
                    let mut counts = std::collections::HashMap::new();
                    for card in &played_cards {
                        *counts.entry(format!("{:?}", card)).or_insert(0) += 1;
                    }
                    for (card, count) in counts {
                        if count > 1 {
                            println!("     {} was played {} times!", card, count);
                        }
                    }

                    panic!("BUG CONFIRMED: Same card played multiple times!");
                }
            }
        }
    }

    println!("\n✅ TEST PASSED: No duplicate cards played");

    // Rollback
    shared.rollback().await?;

    Ok(())
}
