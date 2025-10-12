//! AI player module - handles automated game decisions.
//!
//! This module provides:
//! - AI trait for different AI implementations
//! - RandomPlayer: makes random legal moves (seedable for tests)
//! - AI orchestration helpers
//! - Memory modes and card play history access

pub mod memory;
mod random;
mod trait_def;

pub use memory::{get_round_card_plays, MemoryMode, TrickPlays};
pub use random::RandomPlayer;
use serde_json::Value as JsonValue;
pub use trait_def::{AiError, AiPlayer};

/// AI failure mode - how to handle AI errors/timeouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiFailureMode {
    /// Panic on errors (for tests)
    Panic,
    /// Fall back to random play (for production)
    FallbackRandom,
}

/// Create an AI player from ai_type string and optional config.
///
/// Currently supports:
/// - "random": RandomPlayer with optional seed from config
///
/// Returns None if ai_type is unrecognized.
pub fn create_ai(ai_type: &str, config: Option<&JsonValue>) -> Option<Box<dyn AiPlayer>> {
    match ai_type {
        "random" => {
            let seed = config.and_then(|c| c.get("seed")).and_then(|s| s.as_u64());
            Some(Box::new(RandomPlayer::new(seed)))
        }
        _ => None,
    }
}
