// Tests for player view and game history access.
//
// These tests cover public information accessible to all players,
// including game history for score tables.

use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::repos::player_view;
use backend::AppError;

use crate::support::build_test_state;
use crate::support::factory::create_fresh_lobby_game;
use crate::support::test_utils::test_seed;

#[actix_web::test]
async fn test_game_history_empty_game() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    let game_id = create_fresh_lobby_game(txn, "game_hist_empty").await?;

    let history = player_view::load_game_history(txn, game_id).await?;

    // Rollback the transaction immediately after last DB access
    shared.rollback().await?;

    assert!(history.rounds.is_empty());

    Ok(())
}

#[actix_web::test]
async fn test_game_history_with_rounds() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    use backend::entities::game_rounds::{self, CardTrump};
    use backend::entities::games::{self, GameState, GameVisibility};
    use backend::entities::{round_bids, round_scores};
    use sea_orm::{ActiveModelTrait, NotSet, Set};
    use time::OffsetDateTime;

    // Create a game
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
        rng_seed: Set(Some(test_seed("game_hist_rounds"))),
        current_round: Set(Some(2i16)),
        starting_dealer_pos: Set(Some(0i16)),
        current_trick_no: Set(0i16),
        current_round_id: Set(None),
        lock_version: Set(0),
    }
    .insert(txn)
    .await?;

    // Create round 1 - completed
    let round1 = game_rounds::ActiveModel {
        id: NotSet,
        game_id: Set(game.id),
        round_no: Set(1i16),
        hand_size: Set(13i16),
        dealer_pos: Set(0i16),
        trump: Set(Some(CardTrump::Hearts)),
        created_at: Set(OffsetDateTime::now_utc()),
        completed_at: Set(Some(OffsetDateTime::now_utc())),
    }
    .insert(txn)
    .await?;

    // Add bids for round 1 - seat 2 has highest bid
    for (seat, bid_value) in [(0, 3), (1, 4), (2, 5), (3, 2)] {
        round_bids::ActiveModel {
            id: NotSet,
            round_id: Set(round1.id),
            player_seat: Set(seat as i16),
            bid_value: Set(bid_value as i16),
            bid_order: Set(seat as i16),
            created_at: Set(OffsetDateTime::now_utc()),
        }
        .insert(txn)
        .await?;
    }

    // Add scores for round 1
    for (seat, round_score, total) in [(0, 3, 3), (1, 14, 14), (2, 5, 5), (3, 2, 2)] {
        round_scores::ActiveModel {
            id: NotSet,
            round_id: Set(round1.id),
            player_seat: Set(seat as i16),
            bid_value: Set([3, 4, 5, 2][seat as usize] as i16),
            tricks_won: Set([3, 4, 5, 2][seat as usize] as i16),
            bid_met: Set(true),
            base_score: Set([3, 4, 5, 2][seat as usize] as i16),
            bonus: Set(10i16),
            round_score: Set(round_score as i16),
            total_score_after: Set(total),
            created_at: Set(OffsetDateTime::now_utc()),
        }
        .insert(txn)
        .await?;
    }

    // Create round 2 - in progress (bids complete, no trump yet)
    let round2 = game_rounds::ActiveModel {
        id: NotSet,
        game_id: Set(game.id),
        round_no: Set(2i16),
        hand_size: Set(12i16),
        dealer_pos: Set(1i16),
        trump: Set(None),
        created_at: Set(OffsetDateTime::now_utc()),
        completed_at: Set(None),
    }
    .insert(txn)
    .await?;

    // Add bids for round 2 - seat 3 has highest bid (starting from dealer+1=2)
    for (seat, bid_value) in [(0, 2), (1, 3), (2, 4), (3, 6)] {
        round_bids::ActiveModel {
            id: NotSet,
            round_id: Set(round2.id),
            player_seat: Set(seat as i16),
            bid_value: Set(bid_value as i16),
            bid_order: Set(((seat + 2) % 4) as i16),
            created_at: Set(OffsetDateTime::now_utc()),
        }
        .insert(txn)
        .await?;
    }

    // Load game history
    let history = player_view::load_game_history(txn, game.id).await?;

    // Rollback the transaction immediately after last DB access
    shared.rollback().await?;

    // Verify we have 2 rounds
    assert_eq!(history.rounds.len(), 2);

    // Verify round 1
    let r1 = &history.rounds[0];
    assert_eq!(r1.round_no, 1);
    assert_eq!(r1.dealer_seat, 0);
    assert_eq!(r1.bids, [Some(3), Some(4), Some(5), Some(2)]);
    assert_eq!(r1.trump_selector_seat, Some(2)); // Seat 2 had highest bid (5)
    assert!(r1.trump.is_some());
    assert_eq!(r1.scores[0].round_score, 3);
    assert_eq!(r1.scores[0].cumulative_score, 3);
    assert_eq!(r1.scores[1].round_score, 14);
    assert_eq!(r1.scores[1].cumulative_score, 14);

    // Verify round 2
    let r2 = &history.rounds[1];
    assert_eq!(r2.round_no, 2);
    assert_eq!(r2.dealer_seat, 1);
    assert_eq!(r2.bids, [Some(2), Some(3), Some(4), Some(6)]);
    assert_eq!(r2.trump_selector_seat, Some(3)); // Seat 3 had highest bid (6)
    assert!(r2.trump.is_none()); // Not selected yet
                                 // Scores should be zeros since round not complete
    assert_eq!(r2.scores[0].round_score, 0);

    Ok(())
}

#[actix_web::test]
async fn test_trump_selector_tie_breaking() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;
    let txn = shared.transaction();

    use backend::entities::games::{self, GameState, GameVisibility};
    use backend::entities::{game_rounds, round_bids};
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
        rng_seed: Set(Some(test_seed("trump_sel_tie_break"))),
        current_round: Set(Some(1i16)),
        starting_dealer_pos: Set(Some(2i16)),
        current_trick_no: Set(0i16),
        current_round_id: Set(None),
        lock_version: Set(0),
    }
    .insert(txn)
    .await?;

    let round = game_rounds::ActiveModel {
        id: NotSet,
        game_id: Set(game.id),
        round_no: Set(1i16),
        hand_size: Set(13i16),
        dealer_pos: Set(2i16), // Dealer at seat 2, so bidding starts at seat 3
        trump: Set(None),
        created_at: Set(OffsetDateTime::now_utc()),
        completed_at: Set(None),
    }
    .insert(txn)
    .await?;

    // Bids with tie: seats 3 and 1 both bid 5
    // Bidding order from dealer+1 (seat 3): 3, 0, 1, 2
    // So seat 3 bids first, then 0, then 1 (also bids 5), then 2
    // Seat 3 should win the tie (earliest bidder)
    for (seat, bid_value) in [(3, 5), (0, 3), (1, 5), (2, 4)] {
        round_bids::ActiveModel {
            id: NotSet,
            round_id: Set(round.id),
            player_seat: Set(seat as i16),
            bid_value: Set(bid_value as i16),
            bid_order: Set(((seat + 1) % 4) as i16), // Relative order from dealer
            created_at: Set(OffsetDateTime::now_utc()),
        }
        .insert(txn)
        .await?;
    }

    let history = player_view::load_game_history(txn, game.id).await?;

    // Rollback the transaction immediately after last DB access
    shared.rollback().await?;

    assert_eq!(history.rounds.len(), 1);
    let r = &history.rounds[0];

    // Seat 3 should be trump selector (earliest with bid of 5)
    assert_eq!(r.trump_selector_seat, Some(3));
    assert_eq!(r.bids[3], Some(5));
    assert_eq!(r.bids[1], Some(5));

    Ok(())
}
