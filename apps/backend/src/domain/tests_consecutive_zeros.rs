//! Tests for consecutive zero bids validation rule.
//!
//! According to game rules (docs/game-rules.md):
//! - A player may bid 0, but cannot do so more than three rounds in a row
//! - After three consecutive 0-bids, that player must bid at least 1 in the next round

use crate::domain::bidding::validate_consecutive_zero_bids;
use crate::domain::player_view::{GameHistory, RoundHistory, RoundScoreDetail};
use crate::errors::domain::{DomainError, ValidationKind};

/// Helper to create a round with specified bids
fn make_round(round_no: u8, bids: [Option<u8>; 4]) -> RoundHistory {
    // Calculate hand size for this round
    let hand_size = crate::domain::hand_size_for_round(round_no).unwrap_or(13);

    RoundHistory {
        round_no,
        hand_size,
        dealer_seat: 0,
        bids,
        trump_selector_seat: None,
        trump: None,
        scores: [
            RoundScoreDetail {
                round_score: 0,
                cumulative_score: 0,
            },
            RoundScoreDetail {
                round_score: 0,
                cumulative_score: 0,
            },
            RoundScoreDetail {
                round_score: 0,
                cumulative_score: 0,
            },
            RoundScoreDetail {
                round_score: 0,
                cumulative_score: 0,
            },
        ],
    }
}

#[test]
fn test_allow_zero_bid_in_first_three_rounds() {
    // Round 1-3: Player 0 can bid 0 without restriction
    let history = GameHistory {
        rounds: vec![
            make_round(1, [Some(0), Some(1), Some(2), Some(3)]),
            make_round(2, [Some(0), Some(2), Some(1), Some(3)]),
        ],
    };

    // Should allow 0 bid in round 3 (only 2 previous rounds)
    let result = validate_consecutive_zero_bids(&history, 0, 3);
    assert!(result.is_ok(), "Should allow 0 bid in round 3");
}

#[test]
fn test_allow_third_consecutive_zero_bid() {
    // Player 0 has bid 0 in rounds 1 and 2, can still bid 0 in round 3 (third consecutive)
    let history = GameHistory {
        rounds: vec![
            make_round(1, [Some(0), Some(1), Some(2), Some(3)]),
            make_round(2, [Some(0), Some(2), Some(1), Some(3)]),
        ],
    };

    // Should allow 0 bid in round 3 (would be third consecutive, which is allowed)
    let result = validate_consecutive_zero_bids(&history, 0, 3);
    assert!(result.is_ok(), "Should allow third consecutive 0 bid");
}

#[test]
fn test_reject_fourth_consecutive_zero_bid() {
    // Player 0 has bid 0 in rounds 1, 2, and 3
    let history = GameHistory {
        rounds: vec![
            make_round(1, [Some(0), Some(1), Some(2), Some(3)]),
            make_round(2, [Some(0), Some(2), Some(1), Some(3)]),
            make_round(3, [Some(0), Some(1), Some(2), Some(3)]),
        ],
    };

    // Should reject 0 bid in round 4 (would be 4th consecutive)
    let result = validate_consecutive_zero_bids(&history, 0, 4);
    assert!(result.is_err(), "Should reject fourth consecutive 0 bid");

    if let Err(err) = result {
        assert!(matches!(
            err,
            DomainError::Validation(ValidationKind::InvalidBid, _)
        ));
    }
}

#[test]
fn test_allow_zero_after_non_zero_bid() {
    // Player 0 bid 0, 0, 1 (non-zero breaks the streak), then can bid 0 again
    let history = GameHistory {
        rounds: vec![
            make_round(1, [Some(0), Some(1), Some(2), Some(3)]),
            make_round(2, [Some(0), Some(2), Some(1), Some(3)]),
            make_round(3, [Some(1), Some(1), Some(2), Some(3)]), // Non-zero
        ],
    };

    // Should allow 0 bid in round 4 (streak was broken)
    let result = validate_consecutive_zero_bids(&history, 0, 4);
    assert!(result.is_ok(), "Should allow 0 bid after streak is broken");
}

#[test]
fn test_different_players_tracked_independently() {
    // Player 0 has 3 consecutive zeros
    // Player 1 has only 1 zero
    let history = GameHistory {
        rounds: vec![
            make_round(1, [Some(0), Some(1), Some(2), Some(3)]),
            make_round(2, [Some(0), Some(0), Some(1), Some(3)]),
            make_round(3, [Some(0), Some(2), Some(2), Some(3)]),
        ],
    };

    // Player 0 should be rejected
    let result_p0 = validate_consecutive_zero_bids(&history, 0, 4);
    assert!(
        result_p0.is_err(),
        "Player 0 should be rejected (3 consecutive)"
    );

    // Player 1 should be allowed (only 1 zero in last 3)
    let result_p1 = validate_consecutive_zero_bids(&history, 1, 4);
    assert!(
        result_p1.is_ok(),
        "Player 1 should be allowed (only 1 zero)"
    );

    // Player 2 should be allowed (no zeros)
    let result_p2 = validate_consecutive_zero_bids(&history, 2, 4);
    assert!(result_p2.is_ok(), "Player 2 should be allowed (no zeros)");
}

#[test]
fn test_allow_after_reset() {
    // Player 0: rounds 1-3 bid 0 (rejected in round 4)
    // Then bids non-zero in round 4
    // Then can bid 0 again in rounds 5-7
    let history = GameHistory {
        rounds: vec![
            make_round(1, [Some(0), Some(1), Some(2), Some(3)]),
            make_round(2, [Some(0), Some(2), Some(1), Some(3)]),
            make_round(3, [Some(0), Some(1), Some(2), Some(3)]),
            make_round(4, [Some(2), Some(1), Some(1), Some(3)]), // Player 0 bids 2
            make_round(5, [Some(0), Some(1), Some(2), Some(3)]), // Can bid 0 again
            make_round(6, [Some(0), Some(2), Some(1), Some(3)]),
        ],
    };

    // Should allow third 0 in round 7 (looking at rounds 4-6: 2, 0, 0)
    let result = validate_consecutive_zero_bids(&history, 0, 7);
    assert!(
        result.is_ok(),
        "Should allow 0 after non-zero bid reset the streak"
    );
}

#[test]
fn test_late_game_enforcement() {
    // Test that rule still applies in later rounds (round 20+)
    let mut rounds = vec![];

    // Rounds 1-19: Player 0 bids non-zero
    for i in 1..=19 {
        rounds.push(make_round(i, [Some(1), Some(1), Some(1), Some(1)]));
    }

    // Rounds 20-22: Player 0 bids 0
    rounds.push(make_round(20, [Some(0), Some(1), Some(1), Some(1)]));
    rounds.push(make_round(21, [Some(0), Some(1), Some(1), Some(1)]));
    rounds.push(make_round(22, [Some(0), Some(1), Some(1), Some(1)]));

    let history = GameHistory { rounds };

    // Should reject 0 bid in round 23
    let result = validate_consecutive_zero_bids(&history, 0, 23);
    assert!(
        result.is_err(),
        "Rule should still apply in late game (round 23)"
    );
}

#[test]
fn test_only_looks_at_last_three_rounds() {
    // Player 0 bid 0 in rounds 1-5 (if hypothetically allowed)
    // But we only look at last 3 rounds
    let history = GameHistory {
        rounds: vec![
            make_round(1, [Some(0), Some(1), Some(2), Some(3)]),
            make_round(2, [Some(0), Some(2), Some(1), Some(3)]),
            make_round(3, [Some(0), Some(1), Some(2), Some(3)]),
            make_round(4, [Some(0), Some(1), Some(1), Some(3)]), // Would have been rejected
            make_round(5, [Some(1), Some(2), Some(1), Some(3)]), // Breaks streak in round 5
        ],
    };

    // Looking at round 6, last 3 are: round 3 (0), round 4 (0), round 5 (1)
    // Only 2 consecutive zeros, so should allow
    let result = validate_consecutive_zero_bids(&history, 0, 6);
    assert!(
        result.is_ok(),
        "Should only look at last 3 rounds, not entire history"
    );
}

#[test]
fn test_incomplete_history_allows_bid() {
    // Edge case: history has fewer than 3 rounds
    let history = GameHistory {
        rounds: vec![make_round(1, [Some(0), Some(1), Some(2), Some(3)])],
    };

    // Should allow 0 bid in round 2 (not enough history)
    let result = validate_consecutive_zero_bids(&history, 0, 2);
    assert!(result.is_ok(), "Should allow when history has < 3 rounds");
}
