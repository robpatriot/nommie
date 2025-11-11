//! AI player module - handles automated game decisions.
//!
//! This module provides the core infrastructure for AI players in Nommie, including:
//! - **[`AiPlayer`]** trait - the interface all AIs must implement
//! - **[`RandomPlayer`]** - reference implementation that makes random legal moves
//! - **[`AiConfig`]** - configuration handling with seed and custom fields
//! - **[`AiError`]** - error types for AI decision-making
//! - **Memory system** - access to historical card plays (for advanced AIs)
//!
//! # For AI Developers
//!
//! If you're building a custom AI player, start with the comprehensive
//! [AI Implementation Guide](../../../../../docs/ai-implementation-guide.md).
//!
//! ## Quick Overview
//!
//! 1. **Implement the [`AiPlayer`] trait** - three decision methods:
//!    - [`AiPlayer::choose_bid`] - select a bid value during bidding
//!    - [`AiPlayer::choose_play`] - select a card to play during tricks
//!    - [`AiPlayer::choose_trump`] - select trump suit after winning bid
//!
//! 2. **Use game state helpers** - always query legal moves:
//!    - [`CurrentRoundInfo::legal_bids()`](crate::domain::player_view::CurrentRoundInfo::legal_bids)
//!    - [`CurrentRoundInfo::legal_plays()`](crate::domain::player_view::CurrentRoundInfo::legal_plays)
//!
//! 3. **Handle errors properly** - return [`AiError`], never panic
//!
//! 4. **Ensure thread safety** - your struct must be `Send + Sync`
//!
//! ## Reference Implementation
//!
//! See [`RandomPlayer`] for a complete working example demonstrating:
//! - Thread-safe interior mutability with `Mutex`
//! - Deterministic behavior via optional seeding
//! - Proper error handling
//! - Use of legal move helpers
//!
//! ## Example
//!
//! ```rust,ignore
//! use crate::ai::{AiPlayer, AiError};
//! use crate::domain::player_view::CurrentRoundInfo;
//! use crate::domain::{Card, Trump};
//!
//! pub struct SimpleAI;
//!
//! impl AiPlayer for SimpleAI {
//!     fn choose_bid(&self, state: &CurrentRoundInfo) -> Result<u8, AiError> {
//!         let legal_bids = state.legal_bids();
//!         Ok(legal_bids[0])  // Bid minimum
//!     }
//!
//!     fn choose_play(&self, state: &CurrentRoundInfo) -> Result<Card, AiError> {
//!         let legal_plays = state.legal_plays();
//!         Ok(legal_plays[0])  // Play first legal card
//!     }
//!
//!     fn choose_trump(&self, _state: &CurrentRoundInfo) -> Result<Trump, AiError> {
//!         Ok(Trump::Spades)  // Always choose Spades
//!     }
//! }
//! ```
//!
//! # For Game Engine Developers
//!
//! Use [`create_ai()`] to instantiate AI players by type with configuration:
//!
//! ```rust,ignore
//! let config = AiConfig::from_json(Some(&serde_json::json!({"seed": 42})));
//! let ai = create_ai(RandomPlayer::NAME, config)
//!     .expect("Unknown AI type");
//! ```

mod chatgpt_heuristic;
mod config;
pub mod memory;
mod random;
pub mod registry;
mod trait_def;

pub use chatgpt_heuristic::HeuristicV1;
pub use config::AiConfig;
pub use memory::{apply_memory_degradation, get_round_card_plays, MemoryMode, TrickPlays};
pub use random::RandomPlayer;
pub use trait_def::{AiError, AiPlayer};

/// AI failure mode - how to handle AI errors/timeouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiFailureMode {
    /// Panic on errors (for tests)
    Panic,
    /// Fall back to random play (for production)
    FallbackRandom,
}

/// Create an AI player from ai_type string and configuration.
///
/// Currently supports:
/// - "random": RandomPlayer with optional seed from config
///
/// # Example
///
/// ```rust,ignore
/// let config = AiConfig::from_json(profile.config.as_ref());
/// let ai = create_ai(RandomPlayer::NAME, config)
///     .ok_or_else(|| AppError::internal("Unknown AI type"))?;
/// ```
///
/// Returns None if ai_type is unrecognized.
pub fn create_ai(ai_type: &str, config: AiConfig) -> Option<Box<dyn AiPlayer>> {
    if let Some(factory) = registry::by_name(ai_type) {
        return Some((factory.make)(config.seed()));
    }

    None
}
