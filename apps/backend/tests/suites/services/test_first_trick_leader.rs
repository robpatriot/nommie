// Test that verifies the player to the left of the dealer leads the first trick.
//
// This test would have caught the bug in games.rs where turn_start was incorrectly
// set to dealer_pos instead of (dealer_pos + 1) % 4.

use backend::db::txn::with_txn;
use backend::domain::state::Phase;
use backend::error::AppError;
use backend::repos::rounds;
use backend::services::game_flow::GameFlowService;
use backend::services::games::GameService;

use crate::support::build_test_state;
use crate::support::game_phases::setup_game_in_trick_play_phase;
use crate::support::trick_helpers::create_tricks_by_winner_counts;

/// Test: Verify the first trick leader rotates with dealer position across rounds.
///
/// This tests the critical rule from docs/rules.md:
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
                rounds::Trump::Hearts,
            )
            .await?;

            // === ROUND 1: Verify dealer=0, first player=1 ===
            let loaded_state = game_service.load_game_state(txn, setup.game_id).await?;

            assert_eq!(setup.dealer_pos, 0, "Round 1 should have dealer at seat 0");
            assert_eq!(loaded_state.phase, Phase::Trick { trick_no: 1 });
            assert_eq!(
                loaded_state.turn_start, 1,
                "Round 1: dealer=0, turn_start should be 1 (dealer+1)"
            );
            assert_eq!(
                loaded_state.leader, 1,
                "Round 1: dealer=0, leader should be 1 (dealer+1)"
            );
            assert_eq!(
                loaded_state.turn, 1,
                "Round 1: dealer=0, turn should be 1 (dealer+1)"
            );

            // Complete Round 1: create all tricks and score
            // Hand size for round 1 is 13, so we need 13 tricks
            // Winners: [5, 4, 3, 0] to match bids exactly (everyone gets bonus)
            create_tricks_by_winner_counts(txn, setup.round_id, [5, 4, 3, 0]).await?;
            flow_service.score_round(txn, setup.game_id).await?;

            // === ROUND 2: Deal and progress to trick play ===
            flow_service.deal_round(txn, setup.game_id).await?;

            // Round 2 has hand_size=12, dealer should be (0+1)%4=1
            // Submit bids in turn order (starting at dealer+1 = seat 2)
            // Bids: seat 2→3, seat 3→4, seat 0→2, seat 1(dealer)→2
            // Winner: seat 3 with bid=4
            flow_service
                .submit_bid(txn, setup.game_id, 2, 3, None)
                .await?;
            flow_service
                .submit_bid(txn, setup.game_id, 3, 4, None)
                .await?;
            flow_service
                .submit_bid(txn, setup.game_id, 0, 2, None)
                .await?;
            flow_service
                .submit_bid(txn, setup.game_id, 1, 2, None)
                .await?;

            // Set trump (winning bidder is seat 3)
            flow_service
                .set_trump(txn, setup.game_id, 3, rounds::Trump::Spades, None)
                .await?;

            // === ROUND 2: Verify dealer=1, first player=2 ===
            let loaded_state2 = game_service.load_game_state(txn, setup.game_id).await?;

            let game2 = backend::adapters::games_sea::require_game(txn, setup.game_id).await?;
            let dealer2 = game2.dealer_pos().expect("Round 2 should have dealer");

            assert_eq!(
                dealer2, 1,
                "Round 2 should have dealer at seat 1 (rotated from 0)"
            );
            assert_eq!(loaded_state2.phase, Phase::Trick { trick_no: 1 });
            assert_eq!(
                loaded_state2.turn_start, 2,
                "Round 2: dealer=1, turn_start should be 2 (dealer+1)"
            );
            assert_eq!(
                loaded_state2.leader, 2,
                "Round 2: dealer=1, leader should be 2 (dealer+1)"
            );
            assert_eq!(
                loaded_state2.turn, 2,
                "Round 2: dealer=1, turn should be 2 (dealer+1)"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Verify turn_start consistency between domain init and DB loading
#[tokio::test]
async fn test_turn_start_consistency_domain_vs_db() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create game via DB
            let setup = setup_game_in_trick_play_phase(
                txn,
                "first_trick_leader2",
                [3, 3, 4, 2],
                rounds::Trump::Spades,
            )
            .await?;

            let game_service = GameService;
            let loaded_from_db = game_service.load_game_state(txn, setup.game_id).await?;

            // Create equivalent state via domain init
            use backend::domain::state::init_round;
            let hands = [vec![], vec![], vec![], vec![]]; // Empty hands for this test
            let domain_state = init_round(1, 13, hands, 0, [0, 0, 0, 0]);

            // CRITICAL: Both should have same turn_start calculation
            assert_eq!(
                loaded_from_db.turn_start, domain_state.turn_start,
                "DB loading and domain init should produce same turn_start"
            );

            // Both should be dealer + 1
            assert_eq!(
                domain_state.turn_start, 1,
                "Domain init should set turn_start to dealer+1"
            );
            assert_eq!(
                loaded_from_db.turn_start, 1,
                "DB loading should set turn_start to dealer+1"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
