//! Integration tests for card dealing logic - simple deterministic tests
//! that verify the public API works correctly for basic cases.

use std::collections::HashSet;

use crate::domain::dealing::deal_hands;
use crate::domain::Card;

/// Deterministic dealing test with known seed
#[test]
fn test_deterministic_dealing_with_known_seed() {
    let seed: u64 = 42;
    let hand_size: u8 = 5;

    let hands1 = deal_hands(4, hand_size, seed).unwrap();
    let hands2 = deal_hands(4, hand_size, seed).unwrap();

    // Same seed produces identical results
    assert_eq!(hands1, hands2, "Same seed must produce identical hands");

    // Verify first few cards are deterministic (regression guard)
    // For seed=42, hand_size=5, player 0's first card should always be the same
    assert!(!hands1[0].is_empty(), "Player 0 should have cards");
    let first_card = hands1[0][0];

    // On subsequent runs with same seed, first card should match
    let hands3 = deal_hands(4, hand_size, seed).unwrap();
    assert_eq!(
        hands3[0][0], first_card,
        "First card for player 0 must be deterministic for seed={seed}"
    );
}

/// Test that different seeds produce different results
#[test]
fn test_different_seeds_produce_different_hands() {
    let hand_size: u8 = 13;

    let hands1 = deal_hands(4, hand_size, 111).unwrap();
    let hands2 = deal_hands(4, hand_size, 222).unwrap();

    // Different seeds should produce different hands (with extremely high probability)
    assert_ne!(
        hands1, hands2,
        "Different seeds should produce different hands"
    );
}

/// Test validation errors for invalid inputs
#[test]
fn test_dealing_validates_player_count() {
    let result = deal_hands(3, 5, 12345);
    assert!(result.is_err(), "Should reject player_count != 4");

    let result = deal_hands(5, 5, 12345);
    assert!(result.is_err(), "Should reject player_count != 4");
}

#[test]
fn test_dealing_validates_hand_size() {
    // Too small
    assert!(
        deal_hands(4, 1, 12345).is_err(),
        "Should reject hand_size < 2"
    );

    // Too large
    assert!(
        deal_hands(4, 14, 12345).is_err(),
        "Should reject hand_size > 13"
    );

    // Valid boundary cases
    assert!(
        deal_hands(4, 2, 12345).is_ok(),
        "Should accept hand_size = 2"
    );
    assert!(
        deal_hands(4, 13, 12345).is_ok(),
        "Should accept hand_size = 13"
    );
}

#[test]
fn test_hands_are_sorted() {
    let hands = deal_hands(4, 10, 99999).unwrap();

    for (i, hand) in hands.iter().enumerate() {
        let mut sorted = hand.clone();
        sorted.sort();
        assert_eq!(hand, &sorted, "Hand {i} must be sorted");
    }
}

#[test]
fn test_no_duplicate_cards_across_hands() {
    let hands = deal_hands(4, 13, 42).unwrap();

    let mut all_cards: Vec<Card> = Vec::new();
    for hand in &hands {
        all_cards.extend(hand.iter().copied());
    }

    // Check uniqueness
    for i in 0..all_cards.len() {
        for j in (i + 1)..all_cards.len() {
            assert_ne!(
                all_cards[i], all_cards[j],
                "Duplicate card found: {:?}",
                all_cards[i]
            );
        }
    }
}

#[test]
fn test_dealing_produces_correct_hand_sizes() {
    let hands = deal_hands(4, 5, 12345).unwrap();

    // Verify each hand has the correct size
    for hand in &hands {
        assert_eq!(hand.len(), 5, "Each hand must have 5 cards");
    }

    // Verify total cards dealt equals player_count * hand_size
    let total: usize = hands.iter().map(|h| h.len()).sum();
    assert_eq!(total, 20, "Total cards dealt must equal 4 * 5");
}

#[test]
fn test_dealing_hand_uniqueness() {
    let hands = deal_hands(4, 3, 999).unwrap();

    // Verify no card appears in multiple hands
    let mut all_cards: Vec<Card> = Vec::new();
    for hand in &hands {
        all_cards.extend(hand.iter());
    }

    let card_set: HashSet<Card> = all_cards.iter().copied().collect();
    assert_eq!(
        card_set.len(),
        all_cards.len(),
        "No card should appear in multiple hands"
    );
}
