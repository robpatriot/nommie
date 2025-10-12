//! Random AI player - makes random legal moves.

use std::sync::Mutex;

use rand::prelude::*;

use super::trait_def::{AiError, AiPlayer};
use crate::domain::player_view::CurrentRoundInfo;
use crate::domain::{Card, Suit};

/// AI that makes random legal moves.
///
/// Can be seeded for deterministic behavior in tests.
pub struct RandomPlayer {
    rng: Mutex<StdRng>,
}

impl RandomPlayer {
    /// Create a new RandomPlayer.
    ///
    /// - If `seed` is Some, uses that seed for deterministic behavior
    /// - If `seed` is None, uses system entropy for randomness
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
    fn choose_bid(&self, state: &CurrentRoundInfo) -> Result<u8, AiError> {
        let legal_bids = state
            .legal_bids()
            .map_err(|e| AiError::Internal(format!("Failed to get legal bids: {e}")))?;

        if legal_bids.is_empty() {
            return Err(AiError::InvalidMove("No legal bids available".into()));
        }

        let mut rng = self
            .rng
            .lock()
            .map_err(|e| AiError::Internal(format!("RNG lock poisoned: {e}")))?;
        let choice = legal_bids
            .choose(&mut *rng)
            .copied()
            .ok_or_else(|| AiError::Internal("Failed to choose random bid".into()))?;

        Ok(choice)
    }

    fn choose_play(&self, state: &CurrentRoundInfo) -> Result<Card, AiError> {
        let legal_plays = state
            .legal_plays()
            .map_err(|e| AiError::Internal(format!("Failed to get legal plays: {e}")))?;

        if legal_plays.is_empty() {
            return Err(AiError::InvalidMove("No legal plays available".into()));
        }

        let mut rng = self
            .rng
            .lock()
            .map_err(|e| AiError::Internal(format!("RNG lock poisoned: {e}")))?;
        let choice = legal_plays
            .choose(&mut *rng)
            .copied()
            .ok_or_else(|| AiError::Internal("Failed to choose random card".into()))?;

        Ok(choice)
    }

    fn choose_trump(&self, _state: &CurrentRoundInfo) -> Result<Suit, AiError> {
        // Random trump selection from 4 suits (we don't include NoTrump for now)
        let suits = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

        let mut rng = self
            .rng
            .lock()
            .map_err(|e| AiError::Internal(format!("RNG lock poisoned: {e}")))?;
        let choice = suits
            .choose(&mut *rng)
            .copied()
            .ok_or_else(|| AiError::Internal("Failed to choose random trump".into()))?;

        Ok(choice)
    }
}
