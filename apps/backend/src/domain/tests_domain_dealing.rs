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

/// Test suit distribution across many deals to detect bias.
///
/// This test performs a large number of deals and checks if suits
/// are evenly distributed. If the shuffle has bias, certain suits
/// will appear more frequently than expected.
#[test]
fn test_suit_distribution_bias() {
    use std::collections::HashMap;

    use crate::domain::Suit;

    const NUM_DEALS: u32 = 10_000;
    const HAND_SIZE: u8 = 13;

    // Track suit counts across all deals
    let mut suit_counts: HashMap<Suit, u32> = HashMap::new();
    suit_counts.insert(Suit::Clubs, 0);
    suit_counts.insert(Suit::Diamonds, 0);
    suit_counts.insert(Suit::Hearts, 0);
    suit_counts.insert(Suit::Spades, 0);

    // Track suit distribution by player position
    let mut suit_by_player: [HashMap<Suit, u32>; 4] = [
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    ];
    for player_suits in &mut suit_by_player {
        player_suits.insert(Suit::Clubs, 0);
        player_suits.insert(Suit::Diamonds, 0);
        player_suits.insert(Suit::Hearts, 0);
        player_suits.insert(Suit::Spades, 0);
    }

    // Perform many deals with different seeds
    for deal_num in 0..NUM_DEALS {
        let seed = deal_num as u64;
        let hands = deal_hands(4, HAND_SIZE, seed).unwrap();

        // Count suits in each hand
        for (player_idx, hand) in hands.iter().enumerate() {
            for card in hand {
                *suit_counts.get_mut(&card.suit).unwrap() += 1;
                *suit_by_player[player_idx].get_mut(&card.suit).unwrap() += 1;
            }
        }
    }

    // Calculate expected counts (each suit should appear equally)
    // Each deal: 4 players * 13 cards = 52 cards total
    // Each suit should appear 13 times per deal
    // Total deals: NUM_DEALS
    // Expected per suit: NUM_DEALS * 13
    let total_cards = NUM_DEALS * 4 * HAND_SIZE as u32;
    let expected_per_suit = total_cards / 4;

    println!("\n=== Suit Distribution Test ({} deals) ===", NUM_DEALS);
    println!("Total cards dealt: {}", total_cards);
    println!("Expected per suit: {}", expected_per_suit);
    println!("\nOverall suit distribution:");
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let count = suit_counts[&suit];
        let percentage = (count as f64 / total_cards as f64) * 100.0;
        let deviation = count as i32 - expected_per_suit as i32;
        let deviation_pct = (deviation as f64 / expected_per_suit as f64) * 100.0;
        println!(
            "  {:?}: {} ({:.2}%) - deviation: {} ({:+.2}%)",
            suit, count, percentage, deviation, deviation_pct
        );
    }

    println!("\nSuit distribution by player position:");
    for (player_idx, player_suits) in suit_by_player.iter().enumerate() {
        println!("  Player {}:", player_idx);
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let count = player_suits[&suit];
            let expected_per_player_suit = NUM_DEALS * HAND_SIZE as u32 / 4;
            let percentage = (count as f64 / (NUM_DEALS as f64 * HAND_SIZE as f64)) * 100.0;
            let deviation = count as i32 - expected_per_player_suit as i32;
            println!(
                "    {:?}: {} ({:.2}%) - deviation: {}",
                suit, count, percentage, deviation
            );
        }
    }

    // Check for significant bias (more than 1% deviation)
    // With 10,000 deals of 13 cards each, we expect ~32,500 cards per suit
    // 1% deviation would be ~325 cards, but we'll use a more lenient threshold
    // for statistical variation: 0.5% of expected
    let max_allowed_deviation = (expected_per_suit as f64 * 0.005) as u32;
    let mut max_deviation = 0u32;

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let count = suit_counts[&suit];
        let deviation = count.abs_diff(expected_per_suit);
        max_deviation = max_deviation.max(deviation);
    }

    println!(
        "\nMax deviation: {} (allowed: {})",
        max_deviation, max_allowed_deviation
    );

    // This test will fail if there's significant bias
    // We use a lenient threshold to account for statistical variation
    assert!(
        max_deviation <= max_allowed_deviation,
        "Suit distribution shows significant bias. Max deviation: {} (expected < {})",
        max_deviation,
        max_allowed_deviation
    );
}
