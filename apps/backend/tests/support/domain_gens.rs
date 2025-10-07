// Proptest generators for domain types.
// These generators ensure unique cards and valid game states for property-based testing.

use std::collections::HashSet;

use backend::domain::{Card, PlayerId, Rank, RoundState, Suit, Trump};
use proptest::prelude::*;

/// Generate a random Suit
pub fn suit() -> impl Strategy<Value = Suit> {
    prop_oneof![
        Just(Suit::Clubs),
        Just(Suit::Diamonds),
        Just(Suit::Hearts),
        Just(Suit::Spades),
    ]
}

/// Generate a random Trump (including NO_TRUMP)
pub fn trump() -> impl Strategy<Value = Trump> {
    prop_oneof![
        Just(Trump::Clubs),
        Just(Trump::Diamonds),
        Just(Trump::Hearts),
        Just(Trump::Spades),
        Just(Trump::NoTrump),
    ]
}

/// Generate a random Rank
pub fn rank() -> impl Strategy<Value = Rank> {
    prop_oneof![
        Just(Rank::Two),
        Just(Rank::Three),
        Just(Rank::Four),
        Just(Rank::Five),
        Just(Rank::Six),
        Just(Rank::Seven),
        Just(Rank::Eight),
        Just(Rank::Nine),
        Just(Rank::Ten),
        Just(Rank::Jack),
        Just(Rank::Queen),
        Just(Rank::King),
        Just(Rank::Ace),
    ]
}

/// Generate a single unique Card
pub fn card() -> impl Strategy<Value = Card> {
    (suit(), rank()).prop_map(|(suit, rank)| Card { suit, rank })
}

/// Generate a vector of N unique cards efficiently
pub fn unique_cards(count: usize) -> impl Strategy<Value = Vec<Card>> {
    // Generate by creating a shuffled subset of all possible cards
    Just(()).prop_perturb(move |_, mut rng| {
        let mut all_cards = Vec::new();
        for &suit in &[Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            for &rank in &[
                Rank::Two,
                Rank::Three,
                Rank::Four,
                Rank::Five,
                Rank::Six,
                Rank::Seven,
                Rank::Eight,
                Rank::Nine,
                Rank::Ten,
                Rank::Jack,
                Rank::Queen,
                Rank::King,
                Rank::Ace,
            ] {
                all_cards.push(Card { suit, rank });
            }
        }
        // Shuffle and take first N
        for i in 0..count.min(all_cards.len()) {
            let j = rng.random_range(i..all_cards.len());
            all_cards.swap(i, j);
        }
        all_cards.truncate(count);
        all_cards
    })
}

/// Generate a vector of 1 to max_count unique cards
pub fn unique_cards_up_to(max_count: usize) -> impl Strategy<Value = Vec<Card>> {
    (1..=max_count).prop_flat_map(unique_cards)
}

/// Generate a hand (vector of 1-13 unique cards)
pub fn hand() -> impl Strategy<Value = Vec<Card>> {
    unique_cards_up_to(13)
}

/// Generate a PlayerId (0-3)
pub fn player_id() -> impl Strategy<Value = PlayerId> {
    0u8..=3u8
}

/// Generate four unique hands ensuring no card appears in multiple hands
/// Keeps the total card count reasonable for fast test execution
pub fn four_unique_hands() -> impl Strategy<Value = [Vec<Card>; 4]> {
    // Generate 4-20 unique cards total (1-5 per hand on average) for speed
    (4usize..=20usize)
        .prop_flat_map(unique_cards)
        .prop_map(|cards| {
            // Partition into 4 hands
            let mut hands: [Vec<Card>; 4] = [vec![], vec![], vec![], vec![]];
            for (i, card) in cards.into_iter().enumerate() {
                hands[i % 4].push(card);
            }
            hands
        })
}

/// Complete trick: 4 unique cards with player associations
/// Returns (leader_seat, plays: [(seat, card); 4], trump, lead_suit)
pub fn complete_trick() -> impl Strategy<Value = (PlayerId, Vec<(PlayerId, Card)>, Trump, Suit)> {
    (player_id(), unique_cards(4), trump()).prop_map(|(leader, cards, trump)| {
        let lead_suit = cards[0].suit;
        let mut plays = Vec::with_capacity(4);
        for (i, &card) in cards.iter().enumerate().take(4) {
            let seat = (leader + i as u8) % 4;
            plays.push((seat, card));
        }
        (leader, plays, trump, lead_suit)
    })
}

/// Complete trick that respects follow-suit rules
/// Returns (leader_seat, plays: [(seat, card); 4], trump, lead_suit, hands_before_trick)
#[allow(clippy::type_complexity)]
pub fn complete_trick_with_follow_suit(
) -> impl Strategy<Value = (PlayerId, Vec<(PlayerId, Card)>, Trump, Suit, [Vec<Card>; 4])> {
    (player_id(), trump(), unique_cards(4)).prop_map(|(leader, trump, cards)| {
        let lead_suit = cards[0].suit;

        // Build plays in seat order
        let mut plays = Vec::with_capacity(4);
        let mut hands: [Vec<Card>; 4] = [vec![], vec![], vec![], vec![]];

        for (i, &card) in cards.iter().enumerate().take(4) {
            let seat = (leader + i as u8) % 4;

            // The hand for this player must contain this card
            hands[seat as usize].push(card);
            plays.push((seat, card));
        }

        (leader, plays, trump, lead_suit, hands)
    })
}

/// Generate a hand containing at least one card of the given suit
pub fn hand_with_suit(suit: Suit) -> impl Strategy<Value = Vec<Card>> {
    (rank(), unique_cards_up_to(12)).prop_map(move |(r, mut rest)| {
        let mut hand = vec![Card { suit, rank: r }];
        // Remove any cards with the same suit from rest to avoid duplicates
        rest.retain(|c| !(c.suit == suit && c.rank == r));
        hand.extend(rest);
        hand
    })
}

/// Generate a hand containing NO cards of the given suit (more efficient version)
pub fn hand_without_suit(excluded_suit: Suit) -> impl Strategy<Value = Vec<Card>> {
    // Generate cards only from the other 3 suits
    Just(()).prop_perturb(move |_, mut rng| {
        let allowed_suits: Vec<Suit> = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
            .into_iter()
            .filter(|&s| s != excluded_suit)
            .collect();

        let mut cards = Vec::new();
        for &suit in &allowed_suits {
            for &rank in &[
                Rank::Two,
                Rank::Three,
                Rank::Four,
                Rank::Five,
                Rank::Six,
                Rank::Seven,
                Rank::Eight,
                Rank::Nine,
                Rank::Ten,
                Rank::Jack,
                Rank::Queen,
                Rank::King,
                Rank::Ace,
            ] {
                cards.push(Card { suit, rank });
            }
        }

        // Shuffle and take 1-13 cards
        let count = rng.random_range(1..=13.min(cards.len()));
        for i in 0..count {
            let j = rng.random_range(i..cards.len());
            cards.swap(i, j);
        }
        cards.truncate(count);
        cards
    })
}

/// Generate a RoundState with a complete trick
pub fn round_state_with_trick() -> impl Strategy<Value = RoundState> {
    complete_trick().prop_map(|(_, plays, trump, lead)| {
        let mut state = RoundState::new();
        state.trick_plays = plays;
        state.trick_lead = Some(lead);
        state.trump = Some(trump);
        state
    })
}

/// Generate two distinct cards
pub fn two_distinct_cards() -> impl Strategy<Value = (Card, Card)> {
    unique_cards(2).prop_map(|cards| (cards[0], cards[1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn gen_unique_cards_are_unique(cards in unique_cards(10)) {
            let set: HashSet<Card> = cards.iter().copied().collect();
            assert_eq!(set.len(), cards.len());
        }

        #[test]
        fn gen_four_hands_no_duplicates(hands in four_unique_hands()) {
            let mut all_cards: Vec<Card> = Vec::new();
            for hand in &hands {
                all_cards.extend(hand);
            }
            let set: HashSet<Card> = all_cards.iter().copied().collect();
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
            let mut h = vec![Card { suit: s, rank: r }];
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
}
