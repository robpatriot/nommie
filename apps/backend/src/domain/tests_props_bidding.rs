//! Property tests for bidding logic (pure domain, no DB).
//!
//! Ruleset Contract (minimal implementation):
//! - Players bid sequentially starting from dealer + 1
//! - Each player bids exactly once
//! - Bids must be in range [0..=hand_size]
//! - Turn order must be respected
//! - After all bids are placed, highest bidder wins (ties go to earliest bidder)

use proptest::prelude::*;

use crate::domain::bidding::{legal_bids_for_hand_size, place_bid, Bid};
use crate::domain::state::Phase;
use crate::domain::test_prelude;
use crate::errors::domain::{DomainError, ValidationKind};

proptest! {
    #![proptest_config(test_prelude::proptest_config())]

    /// Property: Legal bids are from a known finite set
    /// For any hand size, the set of legal bids is [0..=hand_size].
    #[test]
    fn prop_legal_bids_match_hand_size(
        hand_size in 2u8..=13u8,
    ) {
        use crate::domain::test_state_helpers::init_round;
        let hands = [vec![], vec![], vec![], vec![]];
        let mut state = init_round(1, hand_size, hands, 0, [0; 4]);
        state.phase = Phase::Bidding;

        let legal = legal_bids_for_hand_size(state.hand_size);

        // Should have hand_size + 1 bids (0 through hand_size inclusive)
        prop_assert_eq!(legal.len(), (hand_size + 1) as usize,
            "Legal bids count must be hand_size + 1");

        // All values should be in range [0..=hand_size]
        for bid in &legal {
            prop_assert!(bid.0 <= hand_size,
                "Bid {bid:?} must be <= hand_size {hand_size}");
        }

        // Should include 0 and hand_size
        let values: Vec<u8> = legal.iter().map(|b| b.0).collect();
        prop_assert!(values.contains(&0), "Should include bid of 0");
        prop_assert!(values.contains(&hand_size),
            "Should include bid of {hand_size}");
    }

    /// Property: Illegal bids are rejected
    /// Bids outside the valid range must be rejected.
    #[test]
    fn prop_illegal_bids_rejected(
        hand_size in 2u8..=13u8,
        invalid_bid in 14u8..=255u8, // Well outside valid range
    ) {
        use crate::domain::test_state_helpers::init_round;
        let hands = [vec![], vec![], vec![], vec![]];
        let mut state = init_round(1, hand_size, hands, 0, [0; 4]);
        state.phase = Phase::Bidding;
        state.turn = 0;

        let result = place_bid(&mut state, 0, Bid(invalid_bid), None);

        prop_assert!(result.is_err(),
            "Bid {invalid_bid} should be rejected for hand_size {hand_size}");

        // Verify it's a validation error
        if let Err(DomainError::Validation(kind, _)) = result {
            prop_assert_eq!(kind, ValidationKind::InvalidBid,
                "Should be InvalidBid error");
        } else {
            prop_assert!(false, "Should be a Validation error");
        }
    }

    /// Property: Cannot bid twice
    /// A player who has already bid cannot bid again.
    #[test]
    fn prop_cannot_bid_twice(
        (hand_size, first_bid, second_bid) in prop::strategy::Strategy::prop_flat_map(
            2u8..=13u8,
            |hs| (Just(hs), 0u8..=hs, 0u8..=hs)
        ),
    ) {
        // Bids are generated based on hand_size, so they're always valid

        use crate::domain::test_state_helpers::init_round;
        let hands = [vec![], vec![], vec![], vec![]];
        let mut state = init_round(1, hand_size, hands, 0, [0; 4]);
        state.phase = Phase::Bidding;
        state.turn = 0;
        state.turn_start = 0;

        // First bid succeeds
        let result = place_bid(&mut state, 0, Bid(first_bid), None);
        prop_assert!(result.is_ok(),
            "First bid should succeed");

        // After 3 more players bid, return to player 0
        // But player 0 already has a bid, so they can't bid again
        // (In practice, turn would advance and player 0 wouldn't be in turn)
        // Let's test the explicit check: if we try to set player 0 back in turn
        // and they already have a bid, it should fail
        state.turn = 0;
        let result = place_bid(&mut state, 0, Bid(second_bid), None);

        prop_assert!(result.is_err(),
            "Player cannot bid twice");

        if let Err(DomainError::Validation(kind, _)) = result {
            prop_assert_eq!(kind, ValidationKind::InvalidBid,
                "Should be InvalidBid error when trying to bid twice");
        }
    }

    /// Property: Out of turn bids are rejected
    #[test]
    fn prop_out_of_turn_rejected(
        (hand_size, bid_value) in prop::strategy::Strategy::prop_flat_map(
            2u8..=13u8,
            |hs| (Just(hs), 0u8..=hs)
        ),
        wrong_player in 1u8..=3u8,
    ) {
        // bid_value is generated based on hand_size, so it's always valid

        use crate::domain::test_state_helpers::init_round;
        let hands = [vec![], vec![], vec![], vec![]];
        let mut state = init_round(1, hand_size, hands, 0, [0; 4]);
        state.phase = Phase::Bidding;
        state.turn = 0; // Player 0's turn

        // Try to bid as a different player
        let result = place_bid(&mut state, wrong_player, Bid(bid_value), None);

        prop_assert!(result.is_err(),
            "Out of turn bid should be rejected");

        if let Err(DomainError::Validation(kind, _)) = result {
            prop_assert_eq!(kind, ValidationKind::OutOfTurn,
                "Should be OutOfTurn error");
        }
    }
}

/// Table-driven test: Valid bids for various hand sizes
#[test]
fn test_valid_bids_table() {
    let test_cases = vec![
        (2, vec![0, 1, 2]),
        (5, vec![0, 1, 2, 3, 4, 5]),
        (13, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]),
    ];

    for (hand_size, expected_values) in test_cases {
        use crate::domain::test_state_helpers::init_round;
        let hands = [vec![], vec![], vec![], vec![]];
        let mut state = init_round(1, hand_size, hands, 0, [0; 4]);
        state.phase = Phase::Bidding;

        let legal = legal_bids_for_hand_size(state.hand_size);
        let values: Vec<u8> = legal.iter().map(|b| b.0).collect();

        assert_eq!(
            values, expected_values,
            "Legal bids for hand_size={hand_size} must match expected"
        );
    }
}

/// Table-driven test: Invalid bids for various hand sizes
#[test]
fn test_invalid_bids_table() {
    let test_cases = vec![
        (2, vec![3, 4, 5, 100]),
        (5, vec![6, 7, 8, 100]),
        (13, vec![14, 15, 100]),
    ];

    for (hand_size, invalid_bids) in test_cases {
        for &bid_value in &invalid_bids {
            use crate::domain::test_state_helpers::init_round;
            let hands = [vec![], vec![], vec![], vec![]];
            let mut state = init_round(1, hand_size, hands, 0, [0; 4]);
            state.phase = Phase::Bidding;
            state.turn = 0;

            let result = place_bid(&mut state, 0, Bid(bid_value), None);

            assert!(
                result.is_err(),
                "Bid {bid_value} should be invalid for hand_size={hand_size}"
            );

            match result {
                Err(DomainError::Validation(ValidationKind::InvalidBid, _)) => {
                    // Expected
                }
                _ => panic!("Expected InvalidBid validation error"),
            }
        }
    }
}

/// Test: Bid in wrong phase is rejected
#[test]
fn test_bid_wrong_phase() {
    use crate::domain::test_state_helpers::init_round;
    let hands = [vec![], vec![], vec![], vec![]];
    let mut state = init_round(1, 5, hands, 0, [0; 4]);
    state.phase = Phase::Init; // Not Bidding
    state.turn = 0;

    let result = place_bid(&mut state, 0, Bid(3), None);

    assert!(result.is_err(), "Bid in wrong phase should be rejected");

    match result {
        Err(DomainError::Validation(ValidationKind::PhaseMismatch, _)) => {
            // Expected
        }
        _ => panic!("Expected PhaseMismatch validation error"),
    }
}

/// Test: Turn order is respected in sequence
#[test]
fn test_turn_order_sequence() {
    use crate::domain::test_state_helpers::init_round;
    let hands = [vec![], vec![], vec![], vec![]];
    let mut state = init_round(1, 5, hands, 0, [0; 4]);
    state.phase = Phase::Bidding;
    state.turn = 0;
    state.turn_start = 0;

    // Player 0 bids
    assert!(place_bid(&mut state, 0, Bid(2), None).is_ok());
    assert_eq!(state.turn, 1, "Turn should advance to player 1");

    // Player 1 bids
    assert!(place_bid(&mut state, 1, Bid(3), None).is_ok());
    assert_eq!(state.turn, 2, "Turn should advance to player 2");

    // Player 2 bids
    assert!(place_bid(&mut state, 2, Bid(1), None).is_ok());
    assert_eq!(state.turn, 3, "Turn should advance to player 3");

    // Player 3 bids
    assert!(place_bid(&mut state, 3, Bid(4), None).is_ok());

    // After all bids, should transition to TrumpSelect phase
    assert_eq!(
        state.phase,
        Phase::TrumpSelect,
        "Should transition to TrumpSelect after all bids"
    );

    // Winning bidder should be player 3 (bid 4, highest)
    assert_eq!(
        state.round.winning_bidder,
        Some(3),
        "Player 3 should be the winning bidder"
    );
}

/// Test: Tie in bids goes to earliest bidder
#[test]
fn test_bid_tie_resolution() {
    use crate::domain::test_state_helpers::init_round;
    let hands = [vec![], vec![], vec![], vec![]];
    let mut state = init_round(1, 5, hands, 0, [0; 4]);
    state.phase = Phase::Bidding;
    state.turn = 0;
    state.turn_start = 0;

    // All players bid the same amount
    assert!(place_bid(&mut state, 0, Bid(3), None).is_ok());
    assert!(place_bid(&mut state, 1, Bid(3), None).is_ok());
    assert!(place_bid(&mut state, 2, Bid(3), None).is_ok());
    assert!(place_bid(&mut state, 3, Bid(3), None).is_ok());

    // Earliest bidder (player 0) should win
    assert_eq!(
        state.round.winning_bidder,
        Some(0),
        "Earliest bidder (player 0) should win on tie"
    );
}

/// Test: Highest bidder wins with different start positions
#[test]
fn test_highest_bidder_wins() {
    use crate::domain::test_state_helpers::init_round;

    // Test case 1: Clear winner
    let hands = [vec![], vec![], vec![], vec![]];
    let mut state = init_round(1, 5, hands, 0, [0; 4]);
    state.phase = Phase::Bidding;
    state.turn = 0;
    state.turn_start = 0;

    assert!(place_bid(&mut state, 0, Bid(1), None).is_ok());
    assert!(place_bid(&mut state, 1, Bid(5), None).is_ok()); // Highest
    assert!(place_bid(&mut state, 2, Bid(2), None).is_ok());
    assert!(place_bid(&mut state, 3, Bid(3), None).is_ok());

    assert_eq!(
        state.round.winning_bidder,
        Some(1),
        "Player 1 with bid 5 should win"
    );

    // Test case 2: Last player wins
    let hands = [vec![], vec![], vec![], vec![]];
    let mut state = init_round(1, 5, hands, 0, [0; 4]);
    state.phase = Phase::Bidding;
    state.turn = 0;
    state.turn_start = 0;

    assert!(place_bid(&mut state, 0, Bid(2), None).is_ok());
    assert!(place_bid(&mut state, 1, Bid(3), None).is_ok());
    assert!(place_bid(&mut state, 2, Bid(1), None).is_ok());
    assert!(place_bid(&mut state, 3, Bid(4), None).is_ok()); // Highest

    assert_eq!(
        state.round.winning_bidder,
        Some(3),
        "Player 3 with bid 4 should win"
    );
}
