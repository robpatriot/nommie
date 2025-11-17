//! Random AI player - makes random legal moves.
//!
//! This module provides [`RandomPlayer`], a reference implementation of the [`AiPlayer`](super::AiPlayer) trait.
//! It demonstrates best practices for AI implementations including:
//! - Thread-safe interior mutability using [`std::sync::Mutex`]
//! - Deterministic behavior via optional seeding
//! - Proper error handling without panics
//! - Use of legal move helper methods
//!
//! This implementation serves as a baseline for testing and as a template for custom AIs.

use std::sync::Mutex;

use rand::prelude::*;

use super::trait_def::{AiError, AiPlayer};
use crate::domain::player_view::CurrentRoundInfo;
use crate::domain::{Card, GameContext, Trump};

/// AI that makes random legal moves.
///
/// `RandomPlayer` is the reference implementation of [`AiPlayer`], demonstrating proper
/// patterns for AI development. It chooses uniformly at random from legal moves in all phases.
///
/// # Key Features
///
/// - **Thread-safe**: Uses `Mutex<StdRng>` for interior mutability
/// - **Deterministic**: Optional seed parameter for reproducible behavior
/// - **Robust**: Proper error handling, never panics
/// - **Correct**: Always uses legal move helpers (`legal_bids()`, `legal_plays()`)
///
/// # Usage
///
/// ```rust,ignore
/// use crate::ai::RandomPlayer;
///
/// // Non-deterministic (uses system entropy)
/// let random_ai = RandomPlayer::new(None);
///
/// // Deterministic (uses seed for reproducible behavior)
/// let seeded_ai = RandomPlayer::new(Some(12345));
/// ```
///
/// # As a Reference Implementation
///
/// When building custom AIs, follow these patterns from `RandomPlayer`:
///
/// 1. **Thread Safety**: Use `Mutex` for mutable state (RNG, statistics, etc.)
/// 2. **Legal Moves**: Always query `state.legal_bids()` and `state.legal_plays()`
/// 3. **Error Handling**: Wrap errors, check preconditions, never panic
/// 4. **Determinism**: Support optional seeding for testing
///
/// See the [AI Implementation Guide](docs/ai-player-implementation-guide.md) for details.
pub struct RandomPlayer {
    /// Thread-safe random number generator.
    ///
    /// Wrapped in `Mutex` for interior mutability since `AiPlayer` trait methods
    /// take `&self` (immutable reference) but RNG needs mutable access.
    rng: Mutex<StdRng>,
}

impl RandomPlayer {
    pub const NAME: &'static str = "RandomPlayer";
    pub const VERSION: &'static str = "1.0.0";

    pub const fn name() -> &'static str {
        Self::NAME
    }

    pub const fn version() -> &'static str {
        Self::VERSION
    }

    /// Create a new `RandomPlayer`.
    ///
    /// # Arguments
    ///
    /// * `seed` - Optional RNG seed for deterministic behavior
    ///   - `Some(seed)` - Uses the provided seed for reproducible randomness (useful for testing)
    ///   - `None` - Uses system entropy for true randomness (useful for production)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // For testing with reproducible behavior
    /// let test_ai = RandomPlayer::new(Some(42));
    ///
    /// // For production with true randomness
    /// let prod_ai = RandomPlayer::new(None);
    /// ```
    pub fn new(seed: Option<u64>) -> Self {
        let rng = if let Some(s) = seed {
            StdRng::seed_from_u64(s)
        } else {
            StdRng::from_entropy()
        };
        Self {
            rng: Mutex::new(rng),
        }
    }
}

impl AiPlayer for RandomPlayer {
    fn choose_bid(&self, state: &CurrentRoundInfo, _context: &GameContext) -> Result<u8, AiError> {
        // Pattern 1: Always get legal moves first
        // This handles dealer restriction and turn order automatically
        let legal_bids = state.legal_bids();

        // Pattern 2: Validate preconditions before proceeding
        if legal_bids.is_empty() {
            return Err(AiError::InvalidMove("No legal bids available".into()));
        }

        // Pattern 3: Lock mutable state (RNG) only when needed
        // Error handling: convert poison error to AiError
        let mut rng = self
            .rng
            .lock()
            .map_err(|e| AiError::Internal(format!("RNG lock poisoned: {e}")))?;

        // Pattern 4: Use Result-based APIs, no unwrap/expect
        let choice = legal_bids
            .choose(&mut *rng)
            .copied()
            .ok_or_else(|| AiError::Internal("Failed to choose random bid".into()))?;

        Ok(choice)
    }

    fn choose_play(
        &self,
        state: &CurrentRoundInfo,
        _context: &GameContext,
    ) -> Result<Card, AiError> {
        // Pattern 1: Get legal plays (handles follow-suit rule automatically)
        let legal_plays = state.legal_plays();

        // Pattern 2: Validate we have options
        if legal_plays.is_empty() {
            return Err(AiError::InvalidMove("No legal plays available".into()));
        }

        // Pattern 3: Thread-safe access to mutable state
        let mut rng = self
            .rng
            .lock()
            .map_err(|e| AiError::Internal(format!("RNG lock poisoned: {e}")))?;

        // Pattern 4: Choose from legal plays only, never from raw hand
        let choice = legal_plays
            .choose(&mut *rng)
            .copied()
            .ok_or_else(|| AiError::Internal("Failed to choose random card".into()))?;

        Ok(choice)
    }

    fn choose_trump(
        &self,
        state: &CurrentRoundInfo,
        _context: &GameContext,
    ) -> Result<Trump, AiError> {
        // Pattern 1: Get all legal trump options (5 choices including NoTrump)
        let legal_trumps = state.legal_trumps();

        // Pattern 2: Validate we have options (should always have 5)
        if legal_trumps.is_empty() {
            return Err(AiError::InvalidMove("No legal trumps available".into()));
        }

        // Pattern 3: Thread-safe RNG access
        let mut rng = self
            .rng
            .lock()
            .map_err(|e| AiError::Internal(format!("RNG lock poisoned: {e}")))?;

        // Pattern 4: No unwrap - use ok_or_else
        // Choose randomly from all 5 trump options (4 suits + NoTrump)
        let choice = legal_trumps
            .choose(&mut *rng)
            .copied()
            .ok_or_else(|| AiError::Internal("Failed to choose random trump".into()))?;

        Ok(choice)
    }
}
