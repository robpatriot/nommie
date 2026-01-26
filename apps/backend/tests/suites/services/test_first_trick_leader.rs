// Test that verifies the player to the left of the dealer leads the first trick.
//
// Verifies that next_player is correctly set to (dealer_pos + 1) % 4, ensuring
// the first trick leader rotates clockwise with the dealer position.

use backend::db::txn::with_txn;
use backend::domain::state::{next_player, Phase};
use backend::domain::Trump;
use backend::services::game_flow::GameFlowService;
use backend::services::games::GameService;
use backend::AppError;

use crate::support::build_test_state;
use crate::support::game_phases::{score_round, setup_game_in_trick_play_phase};
use crate::support::trick_helpers::create_tricks_by_winner_counts;

/// Test: Verify the first trick leader rotates with dealer position across rounds.
///
/// This tests the critical rule from docs/game-rules.md:
/// "The player to the left of the dealer leads the first trick of each round."
///
/// The test progresses through 2 complete rounds to verify:
/// - Round 1: dealer=0 → first player=1 (dealer+1)
/// - Round 2: dealer=1 → first player=2 (dealer+1)
///
/// This ensures that as the dealer rotates clockwise, the first trick leader
/// also rotates clockwise, always staying one position to the left of the dealer.
#[tokio::test]
async fn test_first_trick_leader_is_left_of_dealer() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let game_service = GameService;
            let flow_service = GameFlowService;

            // Set up game in Round 1 trick play phase
            let setup = setup_game_in_trick_play_phase(
                txn,
                "first_trick_leader1",
                [4, 5, 3, 0],
                Trump::Hearts,
            )
            .await?;

            // === ROUND 1: Verify dealer=0, first player=1 ===
            let loaded_state = game_service.load_game_state(txn, setup.game_id).await?;

            assert_eq!(setup.dealer_pos, 0, "Round 1 should have dealer at seat 0");
            assert_eq!(loaded_state.phase, Phase::Trick { trick_no: 1 });
            assert_eq!(
                loaded_state.dealer.map(next_player),
                Some(1),
                "Round 1: dealer=0, next player should be 1 (dealer+1)"
            );
            assert_eq!(
                loaded_state.leader,
                Some(1),
                "Round 1: dealer=0, leader should be 1 (dealer+1)"
            );
            assert_eq!(
                loaded_state.turn,
                Some(1),
                "Round 1: dealer=0, turn should be 1 (dealer+1)"
            );

            // Complete Round 1: create all tricks and score
            // Hand size for round 1 is 13, so we need 13 tricks total.
            // Winners by seat: [5, 4, 3, 1] = 13 tricks.
            create_tricks_by_winner_counts(txn, setup.round_id, [5, 4, 3, 1]).await?;
            score_round(txn, setup.game_id).await?;

            // === ROUND 2: Deal and progress to trick play ===
            // `score_round` has already advanced the game and dealt the next round.
            // Round 2 has hand_size=12, dealer should be (0+1)%4=1
            // Submit bids in turn order (starting at dealer+1 = seat 2)
            // Bids: seat 2→3, seat 3→4, seat 0→2, seat 1(dealer)→2
            // Winner: seat 3 with bid=4
            flow_service
                .submit_bid(
                    txn,
                    setup.game_id,
                    2,
                    3,
                    backend::repos::games::require_game(txn, setup.game_id)
                        .await?
                        .version,
                )
                .await?;
            flow_service
                .submit_bid(
                    txn,
                    setup.game_id,
                    3,
                    4,
                    backend::repos::games::require_game(txn, setup.game_id)
                        .await?
                        .version,
                )
                .await?;
            flow_service
                .submit_bid(
                    txn,
                    setup.game_id,
                    0,
                    2,
                    backend::repos::games::require_game(txn, setup.game_id)
                        .await?
                        .version,
                )
                .await?;
            flow_service
                .submit_bid(
                    txn,
                    setup.game_id,
                    1,
                    2,
                    backend::repos::games::require_game(txn, setup.game_id)
                        .await?
                        .version,
                )
                .await?;

            // Set trump (winning bidder is seat 3)
            flow_service
                .set_trump(
                    txn,
                    setup.game_id,
                    3,
                    Trump::Spades,
                    backend::repos::games::require_game(txn, setup.game_id)
                        .await?
                        .version,
                )
                .await?;

            // === ROUND 2: Verify dealer=1, first player=2 ===
            let loaded_state2 = game_service.load_game_state(txn, setup.game_id).await?;

            let game2 = backend::repos::games::find_by_id(txn, setup.game_id)
                .await?
                .expect("Game should exist");
            let dealer2 = game2.dealer_pos().expect("Round 2 should have dealer");

            assert_eq!(
                dealer2, 1,
                "Round 2 should have dealer at seat 1 (rotated from 0)"
            );
            assert_eq!(loaded_state2.phase, Phase::Trick { trick_no: 1 });
            assert_eq!(
                loaded_state2.dealer.map(next_player),
                Some(2),
                "Round 2: dealer=1, next player should be 2 (dealer+1)"
            );
            assert_eq!(
                loaded_state2.leader,
                Some(2),
                "Round 2: dealer=1, leader should be 2 (dealer+1)"
            );
            assert_eq!(
                loaded_state2.turn,
                Some(2),
                "Round 2: dealer=1, turn should be 2 (dealer+1)"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Verify round-start seat consistency between domain init and DB loading
#[tokio::test]
async fn test_next_player_consistency_domain_vs_db() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create game via DB and advance it to TrickPlay
            let setup = setup_game_in_trick_play_phase(
                txn,
                "first_trick_leader2",
                [3, 3, 4, 2],
                Trump::Spades,
            )
            .await?;

            let game_service = GameService;
            let loaded_from_db = game_service.load_game_state(txn, setup.game_id).await?;

            // Create equivalent state via domain/test helper construction
            use crate::support::state_helpers::{make_game_state, MakeGameStateArgs};

            let hands = [vec![], vec![], vec![], vec![]]; // Empty hands for this test
            let domain_state = make_game_state(
                hands,
                MakeGameStateArgs {
                    // We want to mirror "first trick has started" semantics.
                    phase: backend::domain::state::Phase::Trick { trick_no: 1 },
                    round_no: Some(1),
                    hand_size: Some(13),
                    dealer: Some(0),
                    scores_total: [0, 0, 0, 0],
                    ..Default::default()
                },
            );

            // New canonical assertions:
            // - dealer matches
            // - round-start seat (= left of dealer) matches when derived
            // - in first trick, leader should be round-start seat
            let db_dealer = loaded_from_db
                .dealer
                .expect("DB-loaded state should have dealer in Trick phase");
            let domain_dealer = domain_state
                .dealer
                .expect("Domain state should have dealer in Trick phase");

            assert_eq!(
                db_dealer, domain_dealer,
                "DB loading and domain init should produce same dealer"
            );

            let db_round_start = backend::domain::state::next_player(db_dealer);
            let domain_round_start = backend::domain::state::next_player(domain_dealer);

            assert_eq!(
                db_round_start, domain_round_start,
                "DB and domain should derive the same round-start seat (left of dealer)"
            );

            // Both should be dealer + 1. With dealer=0, round-start should be 1.
            assert_eq!(
                domain_round_start, 1,
                "Domain init should derive round-start seat as dealer+1"
            );
            assert_eq!(
                db_round_start, 1,
                "DB loading should derive round-start seat as dealer+1"
            );

            // First trick leader should be left of dealer
            let db_leader = loaded_from_db
                .leader
                .expect("DB-loaded state should have leader in Trick phase");
            assert_eq!(
                db_leader, db_round_start,
                "First trick leader should be left of dealer"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
