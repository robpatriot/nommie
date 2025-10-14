//! AI memory types for card plays within a round.
//!
//! These types model what an AI player remembers about completed tricks
//! in the current round. Memory fidelity degrades based on the AI's
//! memory_level setting (0-100).

use super::cards_types::{Card, Rank, Suit};
use crate::ai::memory::MemoryMode;

/// AI's memory of completed tricks in the current round.
///
/// Only includes completed tricks - the current trick in progress
/// is available through CurrentRoundInfo instead.
#[derive(Debug, Clone)]
pub struct RoundMemory {
    /// The memory mode that produced this data
    pub mode: MemoryMode,
    /// Completed tricks with potentially degraded card information
    pub tricks: Vec<TrickMemory>,
}

impl RoundMemory {
    /// Create a new RoundMemory.
    pub fn new(mode: MemoryMode, tricks: Vec<TrickMemory>) -> Self {
        Self { mode, tricks }
    }

    /// Check if this memory is empty (no completed tricks yet).
    pub fn is_empty(&self) -> bool {
        self.tricks.is_empty()
    }

    /// Get the number of completed tricks remembered.
    pub fn len(&self) -> usize {
        self.tricks.len()
    }
}

/// What an AI remembers about a single completed trick.
#[derive(Debug, Clone)]
pub struct TrickMemory {
    /// Trick number (1 to hand_size)
    pub trick_no: i16,
    /// What the AI remembers about each play (seat, card memory)
    pub plays: Vec<(i16, PlayMemory)>,
}

impl TrickMemory {
    /// Create a new TrickMemory.
    pub fn new(trick_no: i16, plays: Vec<(i16, PlayMemory)>) -> Self {
        Self { trick_no, plays }
    }
}

/// What an AI remembers about a single card play.
///
/// Memory degrades from perfect recall to partial information to
/// complete forgetting, depending on the AI's memory level and
/// the card's importance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayMemory {
    /// Perfect memory: knows the exact card
    Exact(Card),

    /// Partial memory: remembers the suit but not the rank
    ///
    /// Example: "Someone played a heart, but I don't remember which one"
    Suit(Suit),

    /// Weak memory: only remembers if it was a high, medium, or low card
    ///
    /// Example: "Someone played a high card, but I don't remember suit or exact rank"
    RankCategory(RankCategory),

    /// No memory of this play
    Forgotten,
}

impl PlayMemory {
    /// Check if this memory is exact (not degraded).
    pub fn is_exact(&self) -> bool {
        matches!(self, PlayMemory::Exact(_))
    }

    /// Check if this play is completely forgotten.
    pub fn is_forgotten(&self) -> bool {
        matches!(self, PlayMemory::Forgotten)
    }

    /// Get the exact card if memory is perfect, None otherwise.
    pub fn exact_card(&self) -> Option<Card> {
        match self {
            PlayMemory::Exact(card) => Some(*card),
            _ => None,
        }
    }
}

/// Category for card rank when only rough memory remains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankCategory {
    /// High cards: Jack, Queen, King, Ace
    High,
    /// Medium cards: 7, 8, 9, 10
    Medium,
    /// Low cards: 2, 3, 4, 5, 6
    Low,
}

impl RankCategory {
    /// Categorize a rank into high/medium/low.
    pub fn from_rank(rank: Rank) -> Self {
        match rank {
            Rank::Jack | Rank::Queen | Rank::King | Rank::Ace => RankCategory::High,
            Rank::Seven | Rank::Eight | Rank::Nine | Rank::Ten => RankCategory::Medium,
            Rank::Two | Rank::Three | Rank::Four | Rank::Five | Rank::Six => RankCategory::Low,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cards_types::Suit;

    #[test]
    fn test_rank_category_from_rank() {
        assert_eq!(RankCategory::from_rank(Rank::Ace), RankCategory::High);
        assert_eq!(RankCategory::from_rank(Rank::King), RankCategory::High);
        assert_eq!(RankCategory::from_rank(Rank::Ten), RankCategory::Medium);
        assert_eq!(RankCategory::from_rank(Rank::Seven), RankCategory::Medium);
        assert_eq!(RankCategory::from_rank(Rank::Two), RankCategory::Low);
        assert_eq!(RankCategory::from_rank(Rank::Six), RankCategory::Low);
    }

    #[test]
    fn test_play_memory_is_exact() {
        let exact = PlayMemory::Exact(Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        });
        assert!(exact.is_exact());

        let suit = PlayMemory::Suit(Suit::Hearts);
        assert!(!suit.is_exact());

        let forgotten = PlayMemory::Forgotten;
        assert!(!forgotten.is_exact());
    }

    #[test]
    fn test_play_memory_exact_card() {
        let card = Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        };
        let exact = PlayMemory::Exact(card);
        assert_eq!(exact.exact_card(), Some(card));

        let suit = PlayMemory::Suit(Suit::Hearts);
        assert_eq!(suit.exact_card(), None);
    }

    #[test]
    fn test_round_memory_empty() {
        let memory = RoundMemory::new(MemoryMode::Full, vec![]);
        assert!(memory.is_empty());
        assert_eq!(memory.len(), 0);

        let memory_with_tricks =
            RoundMemory::new(MemoryMode::Full, vec![TrickMemory::new(0, vec![])]);
        assert!(!memory_with_tricks.is_empty());
        assert_eq!(memory_with_tricks.len(), 1);
    }
}
