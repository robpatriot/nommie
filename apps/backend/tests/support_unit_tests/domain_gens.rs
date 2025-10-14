use std::collections::HashSet;

use proptest::prelude::*;

use crate::support::domain_gens::*;

proptest! {
    #[test]
    fn gen_unique_cards_are_unique(cards in unique_cards(10)) {
        let set: HashSet<_> = cards.iter().copied().collect();
        assert_eq!(set.len(), cards.len());
    }

    #[test]
    fn gen_four_hands_no_duplicates(hands in four_unique_hands()) {
        let mut all_cards = Vec::new();
        for hand in &hands {
            all_cards.extend(hand.iter().copied());
        }
        let set: HashSet<_> = all_cards.iter().copied().collect();
        assert_eq!(set.len(), all_cards.len());
    }

    #[test]
    fn gen_complete_trick_has_four_plays(trick in complete_trick()) {
        let (_, plays, _, _) = trick;
        assert_eq!(plays.len(), 4);
    }

    #[test]
    fn gen_hand_with_suit_contains_suit(
        s in suit(),
        r in rank(),
        other in unique_cards_up_to(12),
    ) {
        let mut h = vec![backend::domain::Card { suit: s, rank: r }];
        h.extend(other);
        assert!(h.iter().any(|c| c.suit == s));
    }

    #[test]
    fn gen_hand_without_suit_excludes_suit(
        s in suit(),
        h in hand(),
    ) {
        let filtered: Vec<_> = h.into_iter().filter(|c| c.suit != s).collect();
        assert!(!filtered.iter().any(|c| c.suit == s));
    }
}
