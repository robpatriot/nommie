//! AI player module - handles automated game decisions.
//!
//! This module provides:
//! - AI trait for different AI implementations
//! - RandomPlayer: makes random legal moves (seedable for tests)
//! - AI orchestration helpers
//! - Memory modes and card play history access
//! - Typed configuration interface for AI players

mod config;
pub mod memory;
mod random;
mod trait_def;

pub use config::AiConfig;
pub use memory::{get_round_card_plays, MemoryMode, TrickPlays};
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
/// Returns None if ai_type is unrecognized.
pub fn create_ai(ai_type: &str, config: AiConfig) -> Option<Box<dyn AiPlayer>> {
    match ai_type {
        "random" => Some(Box::new(RandomPlayer::new(config.seed()))),
        _ => None,
    }
}
