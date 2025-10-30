//! Integration tests for domain consistency - simple deterministic tests
//! that verify the public API works correctly for basic cases.

use crate::domain::{card_beats, Card, Suit, Trump};

#[test]
fn test_card_beats_consistency_basic() {
    // Test basic case: trump beats non-trump
    let trump_card = Card {
        suit: Suit::Hearts,
        rank: crate::domain::Rank::Two,
    };
    let non_trump_card = Card {
        suit: Suit::Clubs,
        rank: crate::domain::Rank::Ace,
    };
    let lead = Suit::Clubs;
    let trump = Trump::Hearts;

    let trump_beats_non = card_beats(trump_card, non_trump_card, lead, trump);
    let non_beats_trump = card_beats(non_trump_card, trump_card, lead, trump);

    // Trump should beat non-trump, but not vice versa
    assert!(trump_beats_non);
    assert!(!non_beats_trump);
    // They cannot both beat each other
    assert!(!(trump_beats_non && non_beats_trump));
}

#[test]
fn test_card_beats_consistency_lead_suit() {
    // Test lead suit beats off-suit
    let lead_card = Card {
        suit: Suit::Hearts,
        rank: crate::domain::Rank::Two,
    };
    let off_suit_card = Card {
        suit: Suit::Clubs,
        rank: crate::domain::Rank::Ace,
    };
    let lead = Suit::Hearts;
    let trump = Trump::Spades;

    let lead_beats_off = card_beats(lead_card, off_suit_card, lead, trump);
    let off_beats_lead = card_beats(off_suit_card, lead_card, lead, trump);

    // Lead suit should beat off-suit, but not vice versa
    assert!(lead_beats_off);
    assert!(!off_beats_lead);
    // They cannot both beat each other
    assert!(!(lead_beats_off && off_beats_lead));
}

#[test]
fn test_card_beats_consistency_same_suit() {
    // Test same suit: higher rank beats lower rank
    let high_card = Card {
        suit: Suit::Hearts,
        rank: crate::domain::Rank::Ace,
    };
    let low_card = Card {
        suit: Suit::Hearts,
        rank: crate::domain::Rank::Two,
    };
    let lead = Suit::Hearts;
    let trump = Trump::Spades;

    let high_beats_low = card_beats(high_card, low_card, lead, trump);
    let low_beats_high = card_beats(low_card, high_card, lead, trump);

    // Higher rank should beat lower rank, but not vice versa
    assert!(high_beats_low);
    assert!(!low_beats_high);
    // They cannot both beat each other
    assert!(!(high_beats_low && low_beats_high));
}

#[test]
fn test_card_beats_consistency_off_suit_incomparable() {
    // Test off-suit cards are incomparable
    let card_a = Card {
        suit: Suit::Clubs,
        rank: crate::domain::Rank::Ace,
    };
    let card_b = Card {
        suit: Suit::Diamonds,
        rank: crate::domain::Rank::King,
    };
    let lead = Suit::Hearts;
    let trump = Trump::Spades;

    let a_beats_b = card_beats(card_a, card_b, lead, trump);
    let b_beats_a = card_beats(card_b, card_a, lead, trump);

    // Off-suit cards should be incomparable (neither beats the other)
    assert!(!a_beats_b);
    assert!(!b_beats_a);
}
