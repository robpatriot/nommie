//! AI memory modes and card play history access.

use sea_orm::ConnectionTrait;

use crate::domain::cards_parsing::from_stored_format;
use crate::domain::Card;
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

/// Card plays for a single trick.
#[derive(Debug, Clone)]
pub struct TrickPlays {
    pub trick_no: i16,
    pub plays: Vec<(i16, Card)>, // (player_seat, card)
}

/// Get card play history for a round, filtered by memory mode.
///
/// Returns all tricks with their card plays, subject to the memory mode:
/// - Full: returns all tricks with all plays
/// - Partial: currently returns all tricks (TODO: implement memory impairment)
/// - None: returns empty vec
pub async fn get_round_card_plays<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
    memory_mode: MemoryMode,
) -> Result<Vec<TrickPlays>, AppError> {
    match memory_mode {
        MemoryMode::None => {
            // No memory - return empty
            Ok(Vec::new())
        }
        MemoryMode::Full | MemoryMode::Partial { .. } => {
            // Load all tricks for the round
            let all_tricks = tricks::find_all_by_round(conn, round_id).await?;

            let mut result = Vec::new();

            for trick in all_tricks {
                // Load all plays for this trick
                let play_records = plays::find_all_by_trick(conn, trick.id).await?;

                let mut trick_plays = Vec::new();
                for play in play_records {
                    let card = from_stored_format(&play.card.suit, &play.card.rank)?;
                    trick_plays.push((play.player_seat, card));
                }

                result.push(TrickPlays {
                    trick_no: trick.trick_no,
                    plays: trick_plays,
                });
            }

            // TODO: For Partial mode, implement memory impairment
            // This could involve:
            // - Forgetting some cards (replace with None or remove)
            // - Confusing card suits or ranks
            // - Only remembering recent tricks
            // The 'level' parameter can control the degree of impairment

            Ok(result)
        }
    }
}
