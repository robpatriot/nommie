//! AI player trait definition.
//!
//! This module defines the [`AiPlayer`] trait that all AI implementations must satisfy.
//! External developers implement this trait to create custom AI players for Nommie.
//!
//! # For AI Developers
//!
//! See the comprehensive [AI Implementation Guide](../../../../../../docs/ai-implementation-guide.md)
//! for complete documentation including:
//! - Game rules
//! - Available game state and helper methods
//! - Reference implementation (RandomPlayer)
//! - Advanced features (GameHistory)
//! - Error handling patterns
//! - Testing strategies
//!
//! # Quick Example
//!
//! ```rust,ignore
//! use crate::ai::{AiPlayer, AiError};
//! use crate::domain::player_view::CurrentRoundInfo;
//! use crate::domain::{Card, Trump};
//!
//! pub struct MyAI {
//!     // Your AI's state
//! }
//!
//! impl AiPlayer for MyAI {
//!     fn choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError> {
//!         let legal_bids = state.legal_bids()
//!             .map_err(|e| AiError::Internal(format!("{e}")))?;
//!         // Your bidding logic here
//!         Ok(legal_bids[0])
//!     }
//!
//!     fn choose_play(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Card, AiError> {
//!         let legal_plays = state.legal_plays()
//!             .map_err(|e| AiError::Internal(format!("{e}")))?;
//!         // Your card selection logic here
//!         Ok(legal_plays[0])
//!     }
//!
//!     fn choose_trump(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Trump, AiError> {
//!         // Your trump selection logic here
//!         Ok(Trump::Spades)
//!     }
//! }
//! ```

use std::fmt;

use crate::domain::player_view::CurrentRoundInfo;
use crate::domain::{Card, GameContext, Trump};
use crate::error::AppError;

/// Errors that can occur during AI decision-making.
#[derive(Debug)]
pub enum AiError {
    /// AI failed to make a decision within timeout
    Timeout,
    /// AI encountered an internal error
    Internal(String),
    /// AI produced an invalid move
    InvalidMove(String),
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiError::Timeout => write!(f, "AI decision timeout"),
            AiError::Internal(msg) => write!(f, "AI internal error: {msg}"),
            AiError::InvalidMove(msg) => write!(f, "AI invalid move: {msg}"),
        }
    }
}

impl std::error::Error for AiError {}

impl From<AiError> for AppError {
    fn from(err: AiError) -> Self {
        AppError::internal(
            crate::errors::ErrorCode::InternalError,
            "AI operation failed",
            err,
        )
    }
}

/// Trait for AI players.
///
/// All AI implementations must implement this trait. Each method is called by the game engine
/// when it's the AI's turn to make a decision. The AI receives complete visible game state
/// via [`CurrentRoundInfo`] and must return a legal move.
///
/// # Common Parameters
///
/// All three decision methods receive the same parameters:
///
/// * `state: &CurrentRoundInfo` - Complete visible game state for the current round, including:
///   - Your hand, position, and seat
///   - All bids and scores
///   - Current trick state and plays
///   - Helper methods for legal moves (`legal_bids()`, `legal_plays()`, `legal_trumps()`)
///
/// * `context: &GameContext` - Game-wide context including:
///   - Complete game history via `context.game_history()` for strategic analysis
///   - Historical data persisting across all rounds
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` as the game engine may call methods from different threads.
/// Use interior mutability (e.g., [`std::sync::Mutex`]) for any mutable state like RNG.
///
/// # Error Handling
///
/// Return [`AiError::Internal`] for AI logic errors (RNG failures, computation errors).
/// Return [`AiError::InvalidMove`] if somehow an illegal move is produced (shouldn't happen
/// if using the legal move helper methods).
///
/// **Never panic** - always return an error instead.
///
/// # Example
///
/// ```rust,ignore
/// use crate::ai::{AiPlayer, AiError};
/// use crate::domain::player_view::CurrentRoundInfo;
/// use crate::domain::{Card, Trump};
///
/// pub struct SimpleAI;
///
/// impl AiPlayer for SimpleAI {
///     fn choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError> {
///         let legal_bids = state.legal_bids()
///             .map_err(|e| AiError::Internal(format!("{e}")))?;
///         Ok(legal_bids[0])  // Bid the minimum legal value
///     }
///
///     fn choose_play(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Card, AiError> {
///         let legal_plays = state.legal_plays()
///             .map_err(|e| AiError::Internal(format!("{e}")))?;
///         Ok(legal_plays[0])  // Play the first legal card
///     }
///
///     fn choose_trump(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Trump, AiError> {
///         Ok(Trump::Spades)  // Always choose Spades
///     }
/// }
/// ```
///
/// See [`crate::ai::RandomPlayer`] for a complete reference implementation.
pub trait AiPlayer: Send + Sync {
    /// Choose a bid value during the bidding phase.
    ///
    /// Called once per round when it's the AI's turn to bid. The AI must return a legal
    /// bid value (0 to hand_size). The dealer has a special restriction: cannot bid a value
    /// that makes the sum of all bids equal to hand_size.
    ///
    /// # Returns
    ///
    /// * `Ok(u8)` - A legal bid value (query `state.legal_bids()` for valid options)
    /// * `Err(AiError)` - Internal error or no legal bids available
    ///
    /// # Important
    ///
    /// **Always use** `state.legal_bids()` to get valid bid options. This handles:
    /// - Dealer restriction (sum cannot equal hand_size)
    /// - Valid range (0 to hand_size)
    /// - Turn order (returns empty vec if not your turn)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError> {
    ///     let legal_bids = state.legal_bids()
    ///         .map_err(|e| AiError::Internal(format!("{e}")))?;
    ///     
    ///     if legal_bids.is_empty() {
    ///         return Err(AiError::InvalidMove("No legal bids available".into()));
    ///     }
    ///     
    ///     // Choose based on hand strength
    ///     let high_cards = state.hand.iter()
    ///         .filter(|c| matches!(c.rank, Rank::Jack | Rank::Queen | Rank::King | Rank::Ace))
    ///         .count();
    ///     
    ///     let target_bid = (high_cards / 2) as u8;
    ///     
    ///     // Pick closest legal bid
    ///     legal_bids.iter()
    ///         .min_by_key(|&&b| (b as i16 - target_bid as i16).abs())
    ///         .copied()
    ///         .ok_or_else(|| AiError::Internal("Failed to choose bid".into()))
    /// }
    /// ```
    fn choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError>;

    /// Choose a card to play during trick play.
    ///
    /// Called during each trick when it's the AI's turn to play. The AI must return a legal
    /// card from its hand. Must follow suit if able (enforced by `state.legal_plays()`).
    ///
    /// # Returns
    ///
    /// * `Ok(Card)` - A legal card to play (query `state.legal_plays()` for valid cards)
    /// * `Err(AiError)` - Internal error or no legal plays available
    ///
    /// # Important
    ///
    /// **Always use** `state.legal_plays()` to get valid cards. This handles:
    /// - Follow-suit rule (must play lead suit if you have it)
    /// - Turn order (returns empty vec if not your turn)
    /// - Cards still in your hand
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn choose_play(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Card, AiError> {
    ///     let legal_plays = state.legal_plays()
    ///         .map_err(|e| AiError::Internal(format!("{e}")))?;
    ///     
    ///     if legal_plays.is_empty() {
    ///         return Err(AiError::InvalidMove("No legal plays available".into()));
    ///     }
    ///     
    ///     // If leading, play highest card; if following, play lowest
    ///     if state.current_trick_plays.is_empty() {
    ///         Ok(*legal_plays.iter().max().unwrap())
    ///     } else {
    ///         Ok(*legal_plays.iter().min().unwrap())
    ///     }
    /// }
    /// ```
    fn choose_play(&self, state: &CurrentRoundInfo, context: &GameContext)
        -> Result<Card, AiError>;

    /// Choose trump after winning the bid.
    ///
    /// Called when the AI has the highest bid and must select trump for the round.
    /// Can choose from the four suits (Clubs, Diamonds, Hearts, Spades) or NoTrump.
    ///
    /// # Returns
    ///
    /// * `Ok(Trump)` - The chosen trump (one of 4 suits or NoTrump)
    /// * `Err(AiError)` - Internal error in trump selection
    ///
    /// # Important
    ///
    /// **Use** `state.legal_trumps()` to get valid trump options (returns all 5 choices).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn choose_trump(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Trump, AiError> {
    ///     // Count cards per suit
    ///     let mut suit_counts = [
    ///         (Trump::Clubs, 0),
    ///         (Trump::Diamonds, 0),
    ///         (Trump::Hearts, 0),
    ///         (Trump::Spades, 0),
    ///     ];
    ///     
    ///     for card in &state.hand {
    ///         let idx = match card.suit {
    ///             Suit::Clubs => 0,
    ///             Suit::Diamonds => 1,
    ///             Suit::Hearts => 2,
    ///             Suit::Spades => 3,
    ///         };
    ///         suit_counts[idx].1 += 1;
    ///     }
    ///     
    ///     // Choose suit with most cards (or NoTrump if weak hand)
    ///     let (best_trump, best_count) = suit_counts.iter()
    ///         .max_by_key(|(_, count)| count)
    ///         .copied()
    ///         .unwrap_or((Trump::NoTrump, 0));
    ///     
    ///     // If weak in all suits, choose NoTrump
    ///     if best_count < 3 {
    ///         Ok(Trump::NoTrump)
    ///     } else {
    ///         Ok(best_trump)
    ///     }
    /// }
    /// ```
    fn choose_trump(
        &self,
        state: &CurrentRoundInfo,
        context: &GameContext,
    ) -> Result<Trump, AiError>;
}
