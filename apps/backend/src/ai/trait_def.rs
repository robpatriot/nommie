//! AI player trait definition.

use std::fmt;

use crate::domain::player_view::VisibleGameState;
use crate::domain::{Card, Suit};
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
        AppError::internal(format!("AI error: {err}"))
    }
}

/// Trait for AI players.
///
/// Implementations receive the game state visible to a player and must
/// choose a legal action. The AI is responsible for querying legal moves
/// from the game state.
pub trait AiPlayer: Send + Sync {
    /// Choose a bid value.
    ///
    /// The AI should query `state.legal_bids()` to get valid options.
    fn choose_bid(&self, state: &VisibleGameState) -> Result<u8, AiError>;

    /// Choose a card to play.
    ///
    /// The AI should query `state.legal_plays()` to get valid cards.
    fn choose_play(&self, state: &VisibleGameState) -> Result<Card, AiError>;

    /// Choose trump suit.
    ///
    /// The AI can choose any of the 4 suits or NoTrump.
    fn choose_trump(&self, state: &VisibleGameState) -> Result<Suit, AiError>;
}
