include!("../../common/proptest_prelude.rs");
/// Property-based tests for follow-suit legality rules
use std::collections::HashSet;

use backend::domain::Card;
use proptest::prelude::*;

use crate::support::domain_gens;
use crate::support::domain_prop_helpers::legal_moves_helper;

proptest! {
    #![proptest_config(proptest_prelude_config())]

    /// Property: Follow-suit legality
    /// If a hand contains cards of the lead suit, every legal play must be of that suit.
    /// If not, the set of legal plays must be the entire hand.
    #[test]
    fn prop_follow_suit_legality(
        lead_suit in domain_gens::suit(),
        lead_rank in domain_gens::rank(),
        other_cards in domain_gens::unique_cards_up_to(12),
    ) {
        // Build hand with at least one card of lead_suit
        let mut hand_with = vec![Card { suit: lead_suit, rank: lead_rank }];
        // Filter out duplicates
        for card in other_cards {
            if !(card.suit == lead_suit && card.rank == lead_rank) {
                hand_with.push(card);
            }
        }

        let legal = legal_moves_helper(&hand_with, Some(lead_suit));

        // All legal plays must be of the lead suit
        for card in &legal {
            prop_assert_eq!(card.suit, lead_suit,
                "Legal play {:?} must be of lead suit {:?}", card, lead_suit);
        }

        // All cards of the lead suit in hand must be legal
        let lead_cards: Vec<Card> = hand_with.iter()
            .copied()
            .filter(|c| c.suit == lead_suit)
            .collect();
        prop_assert_eq!(legal.len(), lead_cards.len(),
            "Legal moves count must match lead suit cards in hand");
    }

    /// Property: Follow-suit legality (no lead suit cards)
    /// If hand has no cards of the lead suit, all cards in hand are legal.
    #[test]
    fn prop_follow_suit_when_void((lead_suit, hand_without) in domain_gens::suit().prop_flat_map(|s| {
        (Just(s), domain_gens::hand_without_suit(s))
    })) {
        // hand_without is guaranteed to have no cards of lead_suit
        let legal = legal_moves_helper(&hand_without, Some(lead_suit));

        // All cards in hand should be legal
        let mut expected = hand_without.clone();
        expected.sort();
        prop_assert_eq!(legal, expected,
            "When void in lead suit, all hand cards must be legal");
    }

    /// Property: Legal plays subset
    /// Legal plays must always be a subset of the hand, with no duplicates.
    #[test]
    fn prop_legal_plays_subset(
        hand in domain_gens::hand(),
        lead_suit_opt in proptest::option::of(domain_gens::suit()),
    ) {
        let legal = legal_moves_helper(&hand, lead_suit_opt);

        // No duplicates in legal plays
        let legal_set: HashSet<Card> = legal.iter().copied().collect();
        prop_assert_eq!(legal_set.len(), legal.len(),
            "Legal plays must have no duplicates");

        // All legal plays must be in hand
        for card in &legal {
            prop_assert!(hand.contains(card),
                "Legal play {:?} must be in hand", card);
        }
    }
}
