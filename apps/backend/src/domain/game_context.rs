//! Game-wide context and services for AI players.
//!
//! This module provides GameContext, which gives AI players access to
//! game-wide data and services that persist across rounds, separate from
//! the current round state (CurrentRoundInfo).

use super::player_view::GameHistory;

/// Game-wide context and services available to AI players.
///
/// Provides access to historical data and game-wide services that are
/// independent of the current round state. This is passed as a separate
/// parameter to AI decision methods alongside CurrentRoundInfo.
///
/// # For AI Developers
///
/// Use `GameContext` to access game-wide information:
/// - **Game history**: Complete history of all rounds for strategic analysis
/// - **Future**: AI memory, game statistics, opponent models, etc.
///
/// # Example
///
/// ```rust,ignore
/// fn choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError> {
///     // Access game history for strategic decisions
///     let history = context.game_history();
///     
///     // Analyze opponent patterns over recent rounds
///     let recent_rounds = history.rounds.iter().rev().take(5);
///     for round in recent_rounds {
///         // Analyze bidding patterns, trump choices, etc.
///     }
///     
///     // Make informed bid
///     let legal_bids = state.legal_bids()?;
///     Ok(legal_bids[0])
/// }
/// ```
///
/// # Design
///
/// `GameContext` is separate from `CurrentRoundInfo` because:
/// - Different lifecycle: game-wide vs per-round
/// - Different scope: all players/rounds vs single player/round
/// - Extensibility: natural place for future game-wide services
#[derive(Debug, Clone)]
pub struct GameContext {
    /// Complete game history including all rounds with bids, trumps, and scores.
    game_history: GameHistory,
}

impl GameContext {
    /// Create a new GameContext with the given game history.
    ///
    /// This is typically called by the game orchestration layer, not by AI implementations.
    pub fn new(game_history: GameHistory) -> Self {
        Self { game_history }
    }

    /// Access complete game history for strategic analysis.
    ///
    /// Returns all rounds (completed and partially completed current round) with their
    /// bids, trump selections, and scores. This data is cached by the orchestration
    /// layer and updated after each round completes.
    ///
    /// # For AI Developers
    ///
    /// Use this to build advanced strategies that learn from opponent behavior:
    /// - Analyze opponent bidding tendencies (aggressive vs conservative)
    /// - Track trump selection patterns by player
    /// - Adapt strategy based on score differential
    /// - Build statistical models of opponent play
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Calculate opponent's average bid over last 5 rounds
    /// let history = context.game_history();
    /// let opponent_seat = (state.player_seat + 1) % 4;
    ///
    /// let recent_bids: Vec<u8> = history.rounds
    ///     .iter()
    ///     .rev()
    ///     .take(5)
    ///     .filter_map(|r| r.bids[opponent_seat as usize])
    ///     .collect();
    ///
    /// let avg_bid = if !recent_bids.is_empty() {
    ///     recent_bids.iter().sum::<u8>() as f64 / recent_bids.len() as f64
    /// } else {
    ///     0.0
    /// };
    /// ```
    pub fn game_history(&self) -> &GameHistory {
        &self.game_history
    }

    // Future methods:
    // pub fn ai_memory(&self) -> Option<&AiMemory> { ... }
    // pub fn game_stats(&self) -> &GameStats { ... }
}
