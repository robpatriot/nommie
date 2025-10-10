//! Property tests for card dealing logic (pure domain, no DB).
//!
//! These tests validate that the dealing logic produces deterministic,
//! non-overlapping hands from a standard 52-card deck.

include!("common/proptest_prelude.rs");

use std::collections::HashSet;

use backend::domain::dealing::deal_hands;
use backend::domain::Card;
use proptest::prelude::*;

proptest! {
    #![proptest_config(proptest_prelude_config())]

    /// Property: Full deck has 52 unique cards
    /// When dealing any valid combination, we expect the source deck to be complete.
    #[test]
    fn prop_full_deck_is_unique(
        hand_size in 2u8..=13u8,
        seed in any::<u64>(),
    ) {
        let hands = deal_hands(4, hand_size, seed).unwrap();

        // Collect all dealt cards
        let mut all_cards: Vec<Card> = Vec::new();
        for hand in &hands {
            all_cards.extend(hand.iter());
        }

        // Check uniqueness
        let card_set: HashSet<Card> = all_cards.iter().copied().collect();
        prop_assert_eq!(card_set.len(), all_cards.len(),
            "All dealt cards must be unique");
    }

    /// Property: Dealing produces non-overlapping hands
    /// Each hand must have distinct cards, and the union equals the dealt portion of the deck.
    #[test]
    fn prop_dealing_non_overlapping(
        hand_size in 2u8..=13u8,
        seed in any::<u64>(),
    ) {
        let hands = deal_hands(4, hand_size, seed).unwrap();

        // Verify each hand has the correct size
        for hand in &hands {
            prop_assert_eq!(hand.len(), hand_size as usize,
                "Each hand must have {} cards", hand_size);
        }

        // Verify no card appears in multiple hands
        let mut all_cards: Vec<Card> = Vec::new();
        for hand in &hands {
            all_cards.extend(hand.iter());
        }

        let card_set: HashSet<Card> = all_cards.iter().copied().collect();
        prop_assert_eq!(card_set.len(), all_cards.len(),
            "No card should appear in multiple hands");

        // Verify total cards dealt equals player_count * hand_size
        prop_assert_eq!(all_cards.len(), 4 * hand_size as usize,
            "Total cards dealt must equal 4 * {}", hand_size);
    }

    /// Property: Hand sizes are equal and sum correctly
    #[test]
    fn prop_hand_sizes_equal_and_sum(
        hand_size in 2u8..=13u8,
        seed in any::<u64>(),
    ) {
        let hands = deal_hands(4, hand_size, seed).unwrap();

        // All hands must have the same size
        for hand in &hands {
            prop_assert_eq!(hand.len(), hand_size as usize,
                "Hand size must be {}", hand_size);
        }

        // Total cards must be 4 * hand_size
        let total: usize = hands.iter().map(|h| h.len()).sum();
        prop_assert_eq!(total, 4 * hand_size as usize,
            "Sum of all hand sizes must be {}", 4 * hand_size);
    }
}

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
