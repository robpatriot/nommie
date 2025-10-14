//! AI memory modes and card play history access.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sea_orm::ConnectionTrait;

use crate::domain::cards_parsing::from_stored_format;
use crate::domain::round_memory::{PlayMemory, RankCategory, TrickMemory};
use crate::domain::{Card, Rank};
use crate::error::AppError;
use crate::repos::{plays, tricks};

/// Memory mode for AI players - controls access to historical card plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMode {
    /// Full access to all card plays in the round
    Full,
    /// Partial access with memory impairment
    /// Level represents memory quality (0-100, where 100 is perfect)
    Partial { level: i32 },
    /// No access to historical card plays
    None,
}

impl MemoryMode {
    /// Create a MemoryMode from an optional database value.
    ///
    /// - None or 100 -> Full
    /// - 0 -> None
    /// - 1-99 -> Partial with that level
    pub fn from_db_value(level: Option<i32>) -> Self {
        match level {
            None | Some(100) => MemoryMode::Full,
            Some(0) => MemoryMode::None,
            Some(n) if (1..100).contains(&n) => MemoryMode::Partial { level: n },
            Some(_) => MemoryMode::Full, // Invalid values default to Full
        }
    }

    /// Convert to database value.
    pub fn to_db_value(self) -> Option<i32> {
        match self {
            MemoryMode::Full => Some(100),
            MemoryMode::Partial { level } => Some(level),
            MemoryMode::None => Some(0),
        }
    }
}

/// Card plays for a single trick (undegraded format for internal use).
#[derive(Debug, Clone)]
pub struct TrickPlays {
    pub trick_no: i16,
    pub plays: Vec<(i16, Card)>, // (player_seat, card)
}

/// Get card play history for a round, with memory degradation applied.
///
/// Returns completed tricks from the current round, with memory fidelity
/// determined by the memory mode and AI seed:
///
/// - **None (0)**: Returns empty vec (no memory)
/// - **Full (100)**: Returns all tricks with exact cards
/// - **Partial (1-99)**: Returns tricks with degraded memory based on level
///
/// # Memory Degradation
///
/// For partial memory, cards are randomly forgotten based on:
/// - Memory level (higher = better recall)
/// - Card importance (high cards more memorable than low cards)
/// - AI seed (for deterministic behavior)
///
/// Degradation produces:
/// - `PlayMemory::Exact(card)` - Perfect recall
/// - `PlayMemory::Suit(suit)` - Remember suit, forgot rank
/// - `PlayMemory::RankCategory(high/med/low)` - Vague memory
/// - `PlayMemory::Forgotten` - No memory
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `round_id` - ID of the round to load tricks from
/// * `memory_mode` - Memory level (0-100)
/// * `ai_seed` - Optional seed for deterministic degradation
///
/// # Returns
///
/// Vector of completed tricks with potentially degraded card information.
/// Current trick in progress is NOT included (only completed tricks from DB).
pub async fn get_round_card_plays<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
    memory_mode: MemoryMode,
    ai_seed: Option<u64>,
) -> Result<Vec<TrickMemory>, AppError> {
    // No memory mode - return empty
    if matches!(memory_mode, MemoryMode::None) {
        return Ok(Vec::new());
    }

    // Load all completed tricks for the round
    let all_tricks = tricks::find_all_by_round(conn, round_id).await?;
    let mut raw_plays = Vec::new();

    for trick in all_tricks {
        // Load all plays for this trick
        let play_records = plays::find_all_by_trick(conn, trick.id).await?;

        let mut trick_plays = Vec::new();
        for play in play_records {
            let card = from_stored_format(&play.card.suit, &play.card.rank)?;
            trick_plays.push((play.player_seat, card));
        }

        raw_plays.push(TrickPlays {
            trick_no: trick.trick_no,
            plays: trick_plays,
        });
    }

    // Apply memory degradation based on mode and seed
    Ok(apply_memory_degradation(raw_plays, memory_mode, ai_seed))
}

/// Apply memory degradation to raw trick plays based on memory mode.
///
/// Uses AI's seed for deterministic degradation (same AI sees same forgotten cards).
///
/// This is public to allow game orchestration to cache raw plays and degrade them
/// per-AI based on individual memory levels.
pub fn apply_memory_degradation(
    plays: Vec<TrickPlays>,
    memory_mode: MemoryMode,
    seed: Option<u64>,
) -> Vec<TrickMemory> {
    match memory_mode {
        MemoryMode::None => {
            // Should have been caught earlier, but handle gracefully
            Vec::new()
        }
        MemoryMode::Full => {
            // Perfect memory - convert to TrickMemory with Exact cards
            plays
                .into_iter()
                .map(|trick| {
                    let plays = trick
                        .plays
                        .into_iter()
                        .map(|(seat, card)| (seat, PlayMemory::Exact(card)))
                        .collect();
                    TrickMemory::new(trick.trick_no, plays)
                })
                .collect()
        }
        MemoryMode::Partial { level } => {
            // Initialize RNG with seed for deterministic degradation
            let mut rng = if let Some(s) = seed {
                StdRng::seed_from_u64(s)
            } else {
                StdRng::from_entropy()
            };

            plays
                .into_iter()
                .map(|trick| {
                    let plays = trick
                        .plays
                        .into_iter()
                        .map(|(seat, card)| {
                            let memory = degrade_card_memory(&card, level, &mut rng);
                            (seat, memory)
                        })
                        .collect();
                    TrickMemory::new(trick.trick_no, plays)
                })
                .collect()
        }
    }
}

/// Degrade memory of a single card based on memory level.
///
/// Higher memory level = better recall.
/// High-value cards (Aces, Kings) are more memorable than low cards.
fn degrade_card_memory<R: Rng>(card: &Card, level: i32, rng: &mut R) -> PlayMemory {
    // Calculate card importance weight (high cards more memorable)
    let importance_weight = card_importance_weight(card.rank);

    // Calculate probability of exact recall
    // Formula: base_prob * (0.5 + importance * 0.5)
    // At level=50 with Ace (importance=1.0): 50% * (0.5 + 0.5) = 50%
    // At level=50 with 2 (importance=0.4): 50% * (0.5 + 0.2) = 35%
    let base_prob = (level as f64) / 100.0;
    let remember_exactly = base_prob * (0.5 + importance_weight * 0.5);

    if rng.gen_bool(remember_exactly) {
        // Perfect recall
        return PlayMemory::Exact(*card);
    }

    // Degraded memory - what do we still remember?
    let partial_prob = base_prob * 0.7; // Lower threshold for partial memory

    if rng.gen_bool(partial_prob) {
        // Remember suit but not exact rank
        return PlayMemory::Suit(card.suit);
    }

    if level > 30 {
        // Very weak memory: just remember high/medium/low category
        let category = RankCategory::from_rank(card.rank);
        return PlayMemory::RankCategory(category);
    }

    // Complete forgetting
    PlayMemory::Forgotten
}

/// Calculate importance weight for a card rank (0.0 to 1.0).
///
/// Higher ranks are more memorable in card games.
fn card_importance_weight(rank: Rank) -> f64 {
    match rank {
        Rank::Ace => 1.0,
        Rank::King => 0.95,
        Rank::Queen => 0.85,
        Rank::Jack => 0.75,
        Rank::Ten => 0.6,
        Rank::Nine => 0.5,
        Rank::Eight => 0.45,
        Rank::Seven => 0.4,
        Rank::Six => 0.4,
        Rank::Five => 0.4,
        Rank::Four => 0.4,
        Rank::Three => 0.4,
        Rank::Two => 0.4,
    }
}
