//! Player view of game state - what information is visible to a player.
//!
//! This module provides CurrentRoundInfo which represents all information
//! available to a player at their decision point for the current round, including legal moves.
//! It also provides GameHistory for accessing all public game history (bids, trumps, scores).
//!
//! To load these types from the database, use [`crate::repos::player_view::load_current_round_info`]
//! and [`crate::repos::player_view::load_game_history`].

use crate::domain::state::Phase;
use crate::domain::{valid_bid_range, Card, Trump};

/// Helper function to determine who should lead a trick.
///
/// For trick 0: player to left of dealer (dealer_pos + 1) leads.
/// For other tricks: winner of previous trick leads.
pub fn determine_trick_leader(
    trick_no: i16,
    dealer_pos: i16,
    prev_trick_winner: Option<i16>,
) -> Option<i16> {
    if trick_no == 0 {
        // First trick - leader is player to left of dealer
        Some((dealer_pos + 1) % 4)
    } else {
        // Not first trick - leader is winner of previous trick
        prev_trick_winner
    }
}

/// Information visible to a player at a decision point for the current round.
///
/// This is the primary interface between the game engine and AI players. It provides
/// complete visible game state and helper methods to query legal moves.
///
/// Used by both AI players (passed to [`crate::ai::AiPlayer`] trait methods) and
/// to render UI for human players.
///
/// # For AI Developers
///
/// When implementing [`crate::ai::AiPlayer`], you receive this struct in every decision method.
/// It contains everything you can see as a player at that point in the game.
///
/// ## Key Fields
///
/// - **Your hand**: [`hand`](Self::hand) - cards you can currently play
/// - **Current phase**: [`game_state`](Self::game_state) - Bidding, TrumpSelect, or Trick
/// - **Bids**: [`bids`](Self::bids) - who has bid what so far
/// - **Trick state**: [`current_trick_plays`](Self::current_trick_plays) - cards played this trick
/// - **Scores**: [`scores`](Self::scores) - cumulative scores for all players
///
/// ## Helper Methods
///
/// **Always use these** instead of implementing game rules yourself:
///
/// - [`legal_bids()`](Self::legal_bids) - valid bids you can make (handles dealer restriction)
/// - [`legal_plays()`](Self::legal_plays) - valid cards to play (handles follow-suit rule)
/// - [`legal_trumps()`](Self::legal_trumps) - valid trump choices (all 5 options)
///
/// ## Example
///
/// ```rust,ignore
/// fn choose_bid(&self, state: &CurrentRoundInfo) -> Result<u8, AiError> {
///     // Get legal options
///     let legal_bids = state.legal_bids();
///     
///     // Analyze your hand
///     let high_cards = state.hand.iter()
///         .filter(|c| matches!(c.rank, Rank::Jack | Rank::Queen | Rank::King | Rank::Ace))
///         .count();
///     
///     // Make decision
///     let target = (high_cards / 2) as u8;
///     legal_bids.iter()
///         .min_by_key(|&&b| (b as i16 - target as i16).abs())
///         .copied()
///         .ok_or_else(|| AiError::InvalidMove("No legal bids".into()))
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CurrentRoundInfo {
    /// Game ID (for loading additional data like [`crate::domain::player_view::GameHistory`])
    pub game_id: i64,

    /// Your seat position (0-3), determines turn order
    pub player_seat: i16,

    /// Current game phase: Bidding, TrumpSelect, or Trick
    pub game_state: Phase,

    /// Current round number (0-25, there are 26 rounds total)
    pub current_round: i16,

    /// Number of cards each player has this round (varies: 13→2→2→2→3→13)
    pub hand_size: u8,

    /// Dealer position (0-3) - dealer bids last and has special restrictions
    pub dealer_pos: i16,

    /// Your current hand - cards you can play
    ///
    /// This is updated as you play cards. Use [`legal_plays()`](Self::legal_plays)
    /// to determine which cards are legal to play (handles follow-suit rule).
    pub hand: Vec<Card>,

    /// Bids placed so far this round, indexed by seat position (0-3)
    ///
    /// - `Some(bid)` - player has bid that value
    /// - `None` - player hasn't bid yet
    ///
    /// Example: `[Some(3), Some(2), None, None]` means seats 0 and 1 have bid.
    pub bids: [Option<u8>; 4],

    /// Trump suit if determined, None during bidding
    ///
    /// Set after highest bidder makes their choice. Affects trick resolution
    /// (trump cards beat non-trump regardless of rank).
    pub trump: Option<Trump>,

    /// Current trick number (1 to hand_size)
    ///
    /// Each round has exactly `hand_size` tricks.
    pub trick_no: i16,

    /// Cards played in the current trick so far
    ///
    /// Format: `Vec<(seat, card)>` where seat is 0-3
    ///
    /// - Empty at start of trick
    /// - Up to 4 entries when trick is complete
    /// - First entry is the lead card (determines suit to follow)
    pub current_trick_plays: Vec<(i16, Card)>,

    /// Cumulative scores for all players (indexed by seat 0-3)
    ///
    /// Scores from all completed rounds. Current round score not yet included.
    pub scores: [i16; 4],

    /// Player who should lead the current trick
    ///
    /// - `Some(seat)` during TrickPlay phase
    /// - `None` in other phases
    /// - Player to left of dealer leads trick 1, thereafter winner of previous trick leads
    pub trick_leader: Option<i16>,
}

impl CurrentRoundInfo {
    /// Get legal bids for this player.
    ///
    /// Returns valid bid values (0 to hand_size) that this player can make right now.
    /// Automatically handles:
    /// - Dealer restriction (cannot bid value that makes sum equal hand_size)
    /// - Valid range (0 to hand_size)
    /// - Turn order (returns empty vec if not your turn)
    ///
    /// # Returns
    ///
    /// - `Vec<u8>` - List of legal bid values (sorted 0..=hand_size)
    ///   - Empty if not in Bidding phase or not your turn
    ///   - Non-empty list during your turn in bidding
    ///
    /// # For AI Developers
    ///
    /// **Always use this method** instead of implementing bid validation yourself.
    /// Choose from the returned values to ensure legal moves.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let legal_bids = state.legal_bids();
    /// if legal_bids.is_empty() {
    ///     return Err(AiError::InvalidMove("Not my turn to bid".into()));
    /// }
    /// // Choose from legal_bids
    /// let bid = legal_bids[0];
    /// ```
    pub fn legal_bids(&self) -> Vec<u8> {
        if self.game_state != Phase::Bidding {
            return Vec::new();
        }

        // Check if it's this player's turn
        let bid_count = self.bids.iter().filter(|b| b.is_some()).count();
        let expected_seat = (self.dealer_pos + 1 + bid_count as i16) % 4;
        if self.player_seat != expected_seat {
            return Vec::new();
        }

        let mut legal = valid_bid_range(self.hand_size).collect::<Vec<_>>();

        // Dealer restriction: if last to bid, cannot make sum equal hand_size
        if bid_count == 3 {
            let existing_sum: u8 = self.bids.iter().filter_map(|&b| b).sum();
            let forbidden = self.hand_size.saturating_sub(existing_sum);
            legal.retain(|&b| b != forbidden);
        }

        legal
    }

    /// Get legal plays for this player.
    ///
    /// Returns valid cards from your hand that you can play right now.
    /// Automatically handles:
    /// - Follow-suit rule (must play lead suit if you have it)
    /// - Turn order (returns empty vec if not your turn)
    /// - Cards remaining in your hand
    ///
    /// # Returns
    ///
    /// - `Vec<Card>` - List of legal cards to play (arbitrary order)
    ///   - Empty if not in TrickPlay phase or not your turn
    ///   - Subset of your hand if you must follow suit
    ///   - Your entire hand if leading or can't follow suit
    ///
    /// # For AI Developers
    ///
    /// **Always use this method** instead of implementing follow-suit logic yourself.
    /// Choose from the returned cards to ensure legal moves.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let legal_plays = state.legal_plays();
    /// if legal_plays.is_empty() {
    ///     return Err(AiError::InvalidMove("Not my turn to play".into()));
    /// }
    ///
    /// // If leading, play highest card; if following, play lowest
    /// let card = if state.current_trick_plays.is_empty() {
    ///     *legal_plays.iter().max().unwrap()
    /// } else {
    ///     *legal_plays.iter().min().unwrap()
    /// };
    /// ```
    pub fn legal_plays(&self) -> Vec<Card> {
        if !matches!(self.game_state, Phase::Trick { .. }) {
            return Vec::new();
        }

        // Determine whose turn it is
        let play_count = self.current_trick_plays.len();
        let leader_seat = if play_count == 0 {
            // First play of trick - use the computed trick leader
            // (player to left of dealer for trick 0, previous trick winner otherwise)
            self.trick_leader.unwrap_or(0)
        } else {
            // Not first play - follow turn order from first player
            let first_player = self.current_trick_plays[0].0;
            (first_player + play_count as i16) % 4
        };

        if self.player_seat != leader_seat && play_count > 0 {
            // Not our turn
            return Vec::new();
        }

        // Determine legal cards based on lead suit
        let lead_suit = if let Some((_, first_card)) = self.current_trick_plays.first() {
            Some(first_card.suit)
        } else {
            None
        };

        let legal = if let Some(lead) = lead_suit {
            // Must follow suit if possible
            let matching: Vec<Card> = self
                .hand
                .iter()
                .filter(|c| c.suit == lead)
                .copied()
                .collect();
            if !matching.is_empty() {
                matching
            } else {
                // No cards of lead suit - can play anything
                self.hand.clone()
            }
        } else {
            // First play - can play anything
            self.hand.clone()
        };

        legal
    }

    /// Get legal trump choices (all suits + NoTrump).
    ///
    /// Returns all valid trump options. All 5 choices are always legal.
    ///
    /// # Returns
    ///
    /// Always returns `[Clubs, Diamonds, Hearts, Spades, NoTrump]`
    ///
    /// # For AI Developers
    ///
    /// All 5 trump options are always valid, including `Trump::NoTrump`.
    /// The [`crate::ai::AiPlayer::choose_trump`] method returns a `Trump`,
    /// so you can choose any of these options.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Choose from legal_trumps()
    /// let legal_trumps = state.legal_trumps();
    ///
    /// // Count cards per suit to make decision
    /// let mut suit_counts = [0, 0, 0, 0]; // Clubs, Diamonds, Hearts, Spades
    /// for card in &state.hand {
    ///     let idx = card.suit as usize;
    ///     suit_counts[idx] += 1;
    /// }
    ///
    /// // Choose suit with most cards, or NoTrump if weak
    /// let max_count = *suit_counts.iter().max().unwrap();
    /// if max_count >= 4 {
    ///     Trump::Clubs // (or whichever suit has max_count)
    /// } else {
    ///     Trump::NoTrump
    /// }
    /// ```
    pub fn legal_trumps(&self) -> Vec<Trump> {
        vec![
            Trump::Clubs,
            Trump::Diamonds,
            Trump::Hearts,
            Trump::Spades,
            Trump::NoTrump,
        ]
    }
}

/// Complete game history including all rounds with bids, trumps, and scores.
///
/// Provides access to all completed rounds for analysis of opponent behavior,
/// bidding patterns, and strategic adaptation.
///
/// # For AI Developers
///
/// Use this to build advanced AIs that learn from opponent behavior:
/// - Analyze bidding tendencies (aggressive vs conservative)
/// - Track trump selection patterns
/// - Adapt strategy based on score differential
/// - Build statistical models of opponent play
///
/// # Example
///
/// ```rust,ignore
/// // Calculate opponent's average bid
/// let history = GameHistory::load(conn, game_id).await?;
/// let opponent_seat = 1;
///
/// let mut total = 0;
/// let mut count = 0;
/// for round in &history.rounds {
///     if let Some(bid) = round.bids[opponent_seat as usize] {
///         total += bid as i32;
///         count += 1;
///     }
/// }
///
/// let avg_bid = if count > 0 { total as f64 / count as f64 } else { 0.0 };
/// ```
#[derive(Debug, Clone)]
pub struct GameHistory {
    /// All rounds (completed and current) with their details
    pub rounds: Vec<RoundHistory>,
}

/// History of a single round.
///
/// Contains all information about a round: bids, trump choice, and scores.
#[derive(Debug, Clone)]
pub struct RoundHistory {
    /// Round number (1-26)
    pub round_no: i16,

    /// Hand size for this round (number of cards each player had)
    pub hand_size: u8,

    /// Who dealt this round (0-3)
    pub dealer_seat: i16,

    /// Bids by each player (indexed by seat 0-3)
    ///
    /// `None` if player hasn't bid yet (for incomplete rounds)
    pub bids: [Option<u8>; 4],

    /// Who won the bidding and selected trump
    ///
    /// `None` if bidding not complete
    pub trump_selector_seat: Option<i16>,

    /// Trump choice for this round
    ///
    /// `None` if trump not yet selected
    pub trump: Option<Trump>,

    /// Scores for each player (indexed by seat 0-3)
    pub scores: [RoundScoreDetail; 4],
}

/// Score details for a player in a round.
#[derive(Debug, Clone, Copy)]
pub struct RoundScoreDetail {
    /// Points earned this round (+1 per trick, +10 bonus for exact bid)
    pub round_score: i16,

    /// Total score after this round (cumulative)
    pub cumulative_score: i16,
}

impl GameHistory {
    // Loading methods moved to repos::player_view
}
