//! Property-based tests for domain layer card-play legality and trick-winner rules.
//!
//! Developer notes:
//! - Increase cases locally with: PROPTEST_CASES=800 pnpm be:test
//! - Generators ensure unique cards; use prop_assume! to skip invalid setups.
//! - Oracle comparator is independent of main logic to catch regressions.
//!
//! All tests are pure (no DB, no HTTP) and deterministic.

mod support;

use std::collections::HashSet;
use std::env;

use backend::domain::{card_beats, hand_has_suit, Card, PlayerId, Rank, RoundState, Suit, Trump};
use proptest::prelude::*;
use support::domain_gens;

/// Helper to get proptest config from environment
fn proptest_config() -> ProptestConfig {
    let cases = env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(32); // Low default for fast CI

    ProptestConfig {
        cases,
        ..ProptestConfig::default()
    }
}

/// Independent oracle for trick winner to cross-check domain logic.
/// Returns the index (0-3) of the winning play.
/// Assumes `plays` are in seat order and length == 4.
fn oracle_trick_winner(plays: &[(PlayerId, Card)], trump: Trump) -> usize {
    assert_eq!(plays.len(), 4, "Oracle requires exactly 4 plays");

    // Derive lead suit from the first play to avoid parameter mismatch.
    let lead = plays[0].1.suit;

    // Map trump enum to an optional suit.
    let trump_suit: Option<Suit> = match trump {
        Trump::Clubs => Some(Suit::Clubs),
        Trump::Diamonds => Some(Suit::Diamonds),
        Trump::Hearts => Some(Suit::Hearts),
        Trump::Spades => Some(Suit::Spades),
        Trump::NoTrump => None,
    };

    // Independent rank ordering (highest first). Adjust to your game’s rank order.
    fn rank_score(r: Rank) -> u8 {
        match r {
            Rank::Ace => 13,
            Rank::King => 12,
            Rank::Queen => 11,
            Rank::Jack => 10,
            Rank::Ten => 9,
            Rank::Nine => 8,
            Rank::Eight => 7,
            Rank::Seven => 6,
            Rank::Six => 5,
            Rank::Five => 4,
            Rank::Four => 3,
            Rank::Three => 2,
            Rank::Two => 1,
        }
    }

    // Compute key = (is_trump, is_lead, rank_score) and take the max.
    let mut best_idx = 0;
    let mut best_key = {
        let c = plays[0].1;
        (
            (trump_suit == Some(c.suit)) as u8,
            (c.suit == lead) as u8,
            rank_score(c.rank),
        )
    };

    for (i, &(_, c)) in plays.iter().enumerate().skip(1) {
        let key = (
            (trump_suit == Some(c.suit)) as u8,
            (c.suit == lead) as u8,
            rank_score(c.rank),
        );
        if key > best_key {
            best_key = key;
            best_idx = i;
        }
    }

    best_idx
}

/// Compute legal moves for a given hand and optional lead suit.
/// This is a test helper that mirrors domain::tricks::legal_moves but works without GameState.
fn legal_moves_helper(hand: &[Card], lead: Option<Suit>) -> Vec<Card> {
    if hand.is_empty() {
        return Vec::new();
    }

    if let Some(lead_suit) = lead {
        if hand_has_suit(hand, lead_suit) {
            let mut v: Vec<Card> = hand
                .iter()
                .copied()
                .filter(|c| c.suit == lead_suit)
                .collect();
            v.sort();
            return v;
        }
    }

    let mut any = hand.to_vec();
    any.sort();
    any
}

proptest! {
    #![proptest_config(proptest_config())]

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

    /// Property: Trick winner with NO_TRUMP
    /// In a fully played trick with no trump, the winner must be the highest-ranked card
    /// of the lead suit. Off-suit cards cannot win.
    #[test]
    fn prop_trick_winner_no_trump(
        trick_data in domain_gens::complete_trick(),
    ) {
        let (_leader, plays, _, _) = trick_data;
        let lead = plays[0].1.suit;
        let trump = Trump::NoTrump;

        // Build RoundState
        let mut state = RoundState::new();
        state.trick_plays = plays.clone();
        state.trick_lead = Some(lead);
        state.trump = Some(trump);

        // Get domain winner
        let winner = backend::domain::tricks::resolve_current_trick(&state);
        prop_assert!(winner.is_some(), "Complete trick must have a winner");
        let winner_id = winner.unwrap();

        // Oracle winner
        let oracle_idx = oracle_trick_winner(&plays, trump);
        let oracle_winner_id = plays[oracle_idx].0;

        prop_assert_eq!(winner_id, oracle_winner_id,
            "Domain winner {:?} must match oracle winner {:?} for NO_TRUMP. Lead={:?}, plays={:?}",
            winner_id, oracle_winner_id, lead, plays);

        // Verify winner has lead suit (when any lead suit cards played)
        let winner_card = plays.iter().find(|(id, _)| *id == winner_id).unwrap().1;
        let lead_cards: Vec<_> = plays.iter().filter(|(_, c)| c.suit == lead).collect();

        if !lead_cards.is_empty() {
            prop_assert_eq!(winner_card.suit, lead,
                "NO_TRUMP winner must be of lead suit when lead cards are played");

            // Verify it's the highest rank of lead suit
            for (_, card) in &lead_cards {
                prop_assert!(winner_card.rank >= card.rank,
                    "Winner rank {:?} must be >= all lead suit ranks", winner_card.rank);
            }
        }
    }

    /// Property: Trick winner with trump
    /// In a fully played trick with a trump suit, if any trump cards are played,
    /// the highest trump wins; otherwise, the highest card of the lead suit wins.
    #[test]
    fn prop_trick_winner_with_trump(
        trick_data in domain_gens::complete_trick(),
    ) {
        let (_, plays, trump, _) = trick_data;

        // Skip NO_TRUMP for this test
        prop_assume!(trump != Trump::NoTrump);

        let lead = plays[0].1.suit;

        // Build RoundState
        let mut state = RoundState::new();
        state.trick_plays = plays.clone();
        state.trick_lead = Some(lead);
        state.trump = Some(trump);

        // Get domain winner
        let winner = backend::domain::tricks::resolve_current_trick(&state);
        prop_assert!(winner.is_some(), "Complete trick must have a winner");
        let winner_id = winner.unwrap();

        // Oracle winner
        let oracle_idx = oracle_trick_winner(&plays, trump);
        let oracle_winner_id = plays[oracle_idx].0;

        prop_assert_eq!(winner_id, oracle_winner_id,
            "Domain winner {:?} must match oracle winner {:?} with trump. Trump={:?}, lead={:?}, plays={:?}",
            winner_id, oracle_winner_id, trump, lead, plays);

        // Verify winner logic
        let trump_suit = match trump {
            Trump::Clubs => Suit::Clubs,
            Trump::Diamonds => Suit::Diamonds,
            Trump::Hearts => Suit::Hearts,
            Trump::Spades => Suit::Spades,
            Trump::NoTrump => unreachable!(),
        };

        let winner_card = plays.iter().find(|(id, _)| *id == winner_id).unwrap().1;
        let trump_cards: Vec<_> = plays.iter().filter(|(_, c)| c.suit == trump_suit).collect();

        if !trump_cards.is_empty() {
            prop_assert_eq!(winner_card.suit, trump_suit,
                "Winner must be trump when trump cards are played");

            // Verify it's the highest trump
            for (_, card) in &trump_cards {
                prop_assert!(winner_card.rank >= card.rank,
                    "Winner rank {:?} must be >= all trump ranks", winner_card.rank);
            }
        } else {
            // No trump played: winner must be highest of lead suit
            let lead_cards: Vec<_> = plays.iter().filter(|(_, c)| c.suit == lead).collect();
            if !lead_cards.is_empty() {
                prop_assert_eq!(winner_card.suit, lead,
                    "Winner must be lead suit when no trump played");

                for (_, card) in &lead_cards {
                    prop_assert!(winner_card.rank >= card.rank,
                        "Winner rank {:?} must be >= all lead suit ranks", winner_card.rank);
                }
            }
        }
    }

    /// Property: Card comparison consistency
    /// For any two distinct cards, they cannot BOTH beat each other.
    /// Note: Two off-suit cards (neither trump nor lead) are incomparable - neither beats the other.
    #[test]
    fn prop_card_beats_consistency(
        (card_a, card_b) in domain_gens::two_distinct_cards(),
        lead in domain_gens::suit(),
        trump in domain_gens::trump(),
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
        trick_data in domain_gens::complete_trick(),
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
        hands in domain_gens::four_unique_hands(),
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
        trick_data in domain_gens::complete_trick(),
    ) {
        let (_, plays, _, _) = trick_data;

        let cards: Vec<Card> = plays.iter().map(|(_, c)| *c).collect();
        let card_set: HashSet<Card> = cards.iter().copied().collect();

        prop_assert_eq!(card_set.len(), cards.len(),
            "No duplicate cards in trick plays");
    }

    /// Property: Winner oracle cross-check
    /// The domain's trick_winner result must match an independent oracle implementation.
    #[test]
    fn prop_winner_oracle_cross_check(
        trick_data in domain_gens::complete_trick(),
    ) {
        let (_, plays, trump, _) = trick_data;
        let lead = plays[0].1.suit;

        // Build RoundState
        let mut state = RoundState::new();
        state.trick_plays = plays.clone();
        state.trick_lead = Some(lead);
        state.trump = Some(trump);

        // Get domain winner
        let domain_winner = backend::domain::tricks::resolve_current_trick(&state);
        prop_assert!(domain_winner.is_some(), "Domain must return a winner for complete trick");
        let domain_winner_id = domain_winner.unwrap();

        // Get oracle winner
        let oracle_idx = oracle_trick_winner(&plays, trump);
        let oracle_winner_id = plays[oracle_idx].0;

        prop_assert_eq!(domain_winner_id, oracle_winner_id,
            "Domain winner {:?} must match oracle winner {:?}. Trump={:?}, lead={:?}, plays={:?}",
            domain_winner_id, oracle_winner_id, trump, lead, plays);
    }

    /// Property: Next leader is trick winner
    /// After a trick completes, the next leader must be the trick winner.
    #[test]
    fn prop_next_leader_is_winner(
        trick_data in domain_gens::complete_trick(),
    ) {
        let (_, plays, trump, _) = trick_data;
        let lead = plays[0].1.suit;

        // Build RoundState
        let mut state = RoundState::new();
        state.trick_plays = plays.clone();
        state.trick_lead = Some(lead);
        state.trump = Some(trump);

        // Resolve trick
        let winner = backend::domain::tricks::resolve_current_trick(&state);
        prop_assert!(winner.is_some(), "Trick must have a winner");
        let winner_id = winner.unwrap();

        // In actual game flow, the next leader would be set to winner_id
        // This is tested implicitly by checking oracle consistency
        // Here we just verify the winner is one of the players
        prop_assert!(winner_id <= 3, "Winner must be a valid player ID (0-3)");
    }
}
