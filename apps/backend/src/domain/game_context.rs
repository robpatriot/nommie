//! Game context combining game-wide and player-specific state.
//!
//! This module provides GameContext, which unifies game identification,
//! historical data, and optional player-specific round information into
//! a single cohesive structure used by both HTTP handlers and AI systems.

use super::player_view::{CurrentRoundInfo, GameHistory};
use super::round_memory::RoundMemory;
use crate::errors::domain::{DomainError, ValidationKind};

/// Complete game context available at any point in a game.
///
/// Combines game-wide data (id, history) with optional player-specific
/// current round information. Used by both HTTP handlers and AI systems.
///
/// # Progressive Enhancement
///
/// Fields are optional to support different game states:
/// - **Lobby**: Just `game_id`
/// - **Game started**: `game_id` + `history`
/// - **Player action**: `game_id` + `history` + `round_info`
/// - **AI player**: All fields + `round_memory`
///
/// # For AI Developers
///
/// Use `GameContext` to access game-wide information:
/// - **Game history**: Complete history of all rounds for strategic analysis
/// - **Round info**: Current round state from your perspective
/// - **Round memory**: Your AI's memory of played cards (based on memory_level)
///
/// # Example
///
/// ```rust,ignore
/// fn choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError> {
///     // Access game history for strategic decisions
///     let history = context.game_history().ok_or(...)?;
///     
///     // Analyze opponent patterns over recent rounds
///     let recent_rounds = history.rounds.iter().rev().take(5);
///     for round in recent_rounds {
///         // Analyze bidding patterns, trump choices, etc.
///     }
///     
///     // Make informed bid
///     let legal_bids = state.legal_bids();
///     Ok(legal_bids[0])
/// }
/// ```
#[derive(Debug, Clone)]
pub struct GameContext {
    /// Game ID
    ///
    /// Part of the public API for AI players. May be read by external AI implementations.
    #[allow(dead_code)]
    pub game_id: i64,

    /// Complete game history (all rounds, bids, scores)
    ///
    /// Available once game has started (round 1+).
    /// `None` in lobby state before game starts.
    history: Option<GameHistory>,

    /// Current round state from a specific player's perspective
    ///
    /// Only available when context is loaded for a specific player.
    /// Used by HTTP handlers to render UI and by services for player actions.
    ///
    /// Part of the public API for AI players. May be read by external AI implementations.
    #[allow(dead_code)]
    round_info: Option<CurrentRoundInfo>,

    /// AI's memory of completed tricks in the current round
    ///
    /// Filtered by the AI's memory_level setting.
    /// Only present for AI players.
    round_memory: Option<RoundMemory>,
}

impl GameContext {
    /// Create minimal context with just game_id (e.g., lobby state).
    pub fn new(game_id: i64) -> Self {
        Self {
            game_id,
            history: None,
            round_info: None,
            round_memory: None,
        }
    }

    /// Builder: Add game history.
    pub fn with_history(mut self, history: GameHistory) -> Self {
        self.history = Some(history);
        self
    }

    /// Builder: Add player round info.
    ///
    /// Part of the public API for AI players.
    #[allow(dead_code)]
    pub fn with_round_info(mut self, round_info: CurrentRoundInfo) -> Self {
        self.round_info = Some(round_info);
        self
    }

    /// Builder: Add AI round memory.
    pub fn with_round_memory(mut self, round_memory: Option<RoundMemory>) -> Self {
        self.round_memory = round_memory;
        self
    }

    /// Access game history (for validation, UI, AI strategy).
    ///
    /// Returns `None` if game hasn't started yet (lobby state).
    ///
    /// Part of the public API for AI players.
    #[allow(dead_code)]
    pub fn game_history(&self) -> Option<&GameHistory> {
        self.history.as_ref()
    }

    /// Require game history or return error.
    ///
    /// Use this in contexts where history must be present (e.g., mid-game actions).
    ///
    /// Part of the public API for AI players.
    #[allow(dead_code)]
    pub fn require_history(&self) -> Result<&GameHistory, DomainError> {
        self.history.as_ref().ok_or_else(|| {
            DomainError::validation(
                ValidationKind::Other("NO_HISTORY".into()),
                "Game history not available (game may not have started)",
            )
        })
    }

    /// Access player's current round info.
    ///
    /// Returns `None` if context wasn't loaded for a specific player.
    ///
    /// Part of the public API for AI players.
    #[allow(dead_code)]
    pub fn round_info(&self) -> Option<&CurrentRoundInfo> {
        self.round_info.as_ref()
    }

    /// Require round info or return error.
    ///
    /// Use this in contexts where round info must be present (e.g., player actions).
    ///
    /// Part of the public API for AI players.
    #[allow(dead_code)]
    pub fn require_round_info(&self) -> Result<&CurrentRoundInfo, DomainError> {
        self.round_info.as_ref().ok_or_else(|| {
            DomainError::validation(
                ValidationKind::Other("NO_ROUND_INFO".into()),
                "Round info not available",
            )
        })
    }

    /// Access AI's memory of completed tricks in the current round.
    ///
    /// Returns `None` if:
    /// - AI has memory_level = 0 (no memory)
    /// - No tricks have been completed yet in this round
    ///
    /// # For AI Developers
    ///
    /// Use this to make strategic decisions based on what cards have been played
    /// earlier in the round:
    /// - Track which suits opponents are void in
    /// - Remember which high cards have been played
    /// - Build card counting strategies
    ///
    /// Note: Memory fidelity depends on your AI's memory_level setting:
    /// - 100 (Full): Perfect recall of all cards
    /// - 50 (Partial): Some cards forgotten, especially low cards
    /// - 0 (None): No historical memory (returns None)
    ///
    /// The current trick in progress is NOT included here - it's available
    /// via `CurrentRoundInfo.current_trick_plays` instead.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Check if opponent showed void in hearts earlier this round
    /// if let Some(memory) = context.round_memory() {
    ///     for trick in &memory.tricks {
    ///         for (seat, play_memory) in &trick.plays {
    ///             if *seat == opponent_seat {
    ///                 match play_memory {
    ///                     PlayMemory::Exact(card) if card.suit != Suit::Hearts => {
    ///                         // Opponent played non-heart when hearts were led
    ///                         // They're void in hearts!
    ///                     }
    ///                     _ => {}
    ///                 }
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// Part of the public API for AI players.
    #[allow(dead_code)]
    pub fn round_memory(&self) -> Option<&RoundMemory> {
        self.round_memory.as_ref()
    }
}
