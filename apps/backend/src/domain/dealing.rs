//! Deterministic card dealing logic.

use crate::domain::{Card, Rank, Suit};
use crate::errors::domain::{DomainError, ValidationKind};

/// Generate a full 52-card deck in standard order.
fn full_deck() -> Vec<Card> {
    let suits = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
    let ranks = [
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
    ];

    let mut deck = Vec::with_capacity(52);
    for suit in suits {
        for rank in ranks {
            deck.push(Card { suit, rank });
        }
    }
    deck
}

/// Simple deterministic RNG for shuffling.
///
/// Uses a SplitMix64-style generator for good statistical properties while
/// remaining fast and deterministic given a seed.
struct SimpleLcg {
    state: u64,
}

impl SimpleLcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        // SplitMix64: well-distributed 64-bit generator.
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z ^= z >> 30;
        z = z.wrapping_mul(0xBF58476D1CE4E5B9);
        z ^= z >> 27;
        z = z.wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    fn next_range(&mut self, max: usize) -> usize {
        let m = max as u64;
        // Compute largest multiple of m that fits in u64 to avoid modulo bias.
        // Values >= limit are discarded using rejection sampling.
        let limit = u64::MAX - (u64::MAX % m);

        loop {
            let x = self.next();
            if x < limit {
                return (x % m) as usize;
            }
        }
    }
}

/// Fisher-Yates shuffle using deterministic RNG.
fn shuffle_with_seed(deck: &mut [Card], seed: u64) {
    let mut rng = SimpleLcg::new(seed);
    for i in (1..deck.len()).rev() {
        let j = rng.next_range(i + 1);
        deck.swap(i, j);
    }
}

/// Deal hands deterministically given player count, hand size, and RNG seed.
///
/// Returns 4 hands, one per player. Hands are sorted for convenience.
/// Remaining cards are discarded (not needed for the game).
///
/// # Arguments
/// * `player_count` - Number of players (must be 4 for now)
/// * `hand_size` - Cards per player (must be 2..=13)
/// * `seed` - RNG seed for deterministic shuffling
pub fn deal_hands(
    player_count: usize,
    hand_size: u8,
    seed: u64,
) -> Result<[Vec<Card>; 4], DomainError> {
    if player_count != 4 {
        return Err(DomainError::validation(
            ValidationKind::InvalidPlayerCount,
            "Player count must be 4",
        ));
    }

    if !(2..=13).contains(&hand_size) {
        return Err(DomainError::validation(
            ValidationKind::InvalidHandSize,
            "Hand size must be 2..=13",
        ));
    }

    let total_cards = player_count * hand_size as usize;
    if total_cards > 52 {
        return Err(DomainError::validation(
            ValidationKind::InvalidHandSize,
            "Total cards exceed deck size",
        ));
    }

    let mut deck = full_deck();
    shuffle_with_seed(&mut deck, seed);

    let mut hands: [Vec<Card>; 4] = Default::default();
    for (player, hand_slot) in hands.iter_mut().enumerate().take(player_count) {
        let start = player * hand_size as usize;
        let end = start + hand_size as usize;
        let mut hand = deck[start..end].to_vec();
        hand.sort();
        *hand_slot = hand;
    }

    Ok(hands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deal_hands_is_deterministic() {
        let h1 = deal_hands(4, 5, 12345).unwrap();
        let h2 = deal_hands(4, 5, 12345).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn deal_hands_different_seeds_differ() {
        let h1 = deal_hands(4, 5, 12345).unwrap();
        let h2 = deal_hands(4, 5, 54321).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn deal_hands_validates_player_count() {
        let result = deal_hands(3, 5, 12345);
        assert!(result.is_err());
    }

    #[test]
    fn deal_hands_validates_hand_size() {
        assert!(deal_hands(4, 1, 12345).is_err());
        assert!(deal_hands(4, 14, 12345).is_err());
        assert!(deal_hands(4, 2, 12345).is_ok());
        assert!(deal_hands(4, 13, 12345).is_ok());
    }

    #[test]
    fn deal_hands_are_sorted() {
        let hands = deal_hands(4, 13, 99999).unwrap();
        for hand in &hands {
            let mut sorted = hand.clone();
            sorted.sort();
            assert_eq!(hand, &sorted);
        }
    }

    #[test]
    fn deal_hands_no_duplicates() {
        let hands = deal_hands(4, 10, 42).unwrap();
        let mut all_cards: Vec<&Card> = Vec::new();
        for hand in &hands {
            all_cards.extend(hand.iter());
        }
        // Check uniqueness
        for i in 0..all_cards.len() {
            for j in (i + 1)..all_cards.len() {
                assert_ne!(all_cards[i], all_cards[j], "Duplicate card found");
            }
        }
    }
}
