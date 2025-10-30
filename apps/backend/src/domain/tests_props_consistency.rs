//! Property-based tests for system-wide consistency invariants
//! These tests verify that the domain logic maintains consistency across the entire system.

use std::collections::HashSet;

use proptest::prelude::*;

use crate::domain::{card_beats, test_gens, test_prelude, Card, Suit, Trump};

proptest! {
    #![proptest_config(test_prelude::proptest_config())]

    /// Property: Card comparison consistency
    /// For any two distinct cards, they cannot BOTH beat each other.
    /// Note: Two off-suit cards (neither trump nor lead) are incomparable - neither beats the other.
    #[test]
    fn prop_card_beats_consistency(
        (card_a, card_b) in test_gens::two_distinct_cards(),
        lead in test_gens::suit(),
        trump in test_gens::trump(),
    ) {
        let a_beats_b = card_beats(card_a, card_b, lead, trump);
        let b_beats_a = card_beats(card_b, card_a, lead, trump);

        // They cannot both beat each other
        prop_assert!(
            !(a_beats_b && b_beats_a),
            "Cards cannot both beat each other. \
             a={:?}, b={:?}, lead={:?}, trump={:?}, a_beats_b={}, b_beats_a={}",
            card_a, card_b, lead, trump, a_beats_b, b_beats_a
        );

        // Additional check: if one beats the other, they must differ in either trump status,
        // lead status, or rank (within same suit category)
        if a_beats_b || b_beats_a {
            let trump_suit = match trump {
                Trump::Clubs => Some(Suit::Clubs),
                Trump::Diamonds => Some(Suit::Diamonds),
                Trump::Hearts => Some(Suit::Hearts),
                Trump::Spades => Some(Suit::Spades),
                Trump::NoTrump => None,
            };

            let a_trump = trump_suit == Some(card_a.suit);
            let b_trump = trump_suit == Some(card_b.suit);
            let a_lead = card_a.suit == lead;
            let b_lead = card_b.suit == lead;

            // If one beats the other, they must have different properties
            prop_assert!(
                a_trump != b_trump || a_lead != b_lead || card_a.rank != card_b.rank,
                "If one card beats another, they must differ in trump/lead status or rank"
            );
        }
    }

    /// Property: Rotation consistency
    /// For a complete trick starting from a given leader seat, plays must follow
    /// correct seat order (leader, leader+1, leader+2, leader+3 mod 4).
    #[test]
    fn prop_rotation_consistency(
        trick_data in test_gens::complete_trick(),
    ) {
        let (leader, plays, _, _) = trick_data;

        prop_assert_eq!(plays.len(), 4, "Trick must have 4 plays");

        for (i, (seat, _)) in plays.iter().enumerate() {
            let expected_seat = (leader + i as u8) % 4;
            prop_assert_eq!(*seat, expected_seat,
                "Play {} must be by seat {}, got {}", i, expected_seat, seat);
        }
    }

    /// Property: No duplicate cards
    /// Across generated hands and trick plays, each physical card appears at most once.
    #[test]
    fn prop_no_duplicate_cards_in_hands(
        hands in test_gens::four_unique_hands(),
    ) {
        let mut all_cards: Vec<Card> = Vec::new();
        for hand in &hands {
            all_cards.extend(hand);
        }

        let card_set: HashSet<Card> = all_cards.iter().copied().collect();
        prop_assert_eq!(card_set.len(), all_cards.len(),
            "No card should appear in multiple hands");
    }

    /// Property: No duplicate cards in trick
    #[test]
    fn prop_no_duplicate_cards_in_trick(
        trick_data in test_gens::complete_trick(),
    ) {
        let (_, plays, _, _) = trick_data;

        let cards: Vec<Card> = plays.iter().map(|(_, c)| *c).collect();
        let card_set: HashSet<Card> = cards.iter().copied().collect();

        prop_assert_eq!(card_set.len(), cards.len(),
            "No duplicate cards in trick plays");
    }
}
