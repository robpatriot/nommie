//! Player view of game state - what information is visible to a player.
//!
//! This module provides VisibleGameState which represents all information
//! available to a player at their decision point, including legal moves.

use sea_orm::ConnectionTrait;

use crate::domain::cards_parsing::from_stored_format;
use crate::domain::{valid_bid_range, Card, Trump};
use crate::entities::games::GameState as DbGameState;
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::{bids, hands, plays, rounds, tricks};

/// Information visible to a player at a decision point.
///
/// Used by both AI players and to render UI for human players.
#[derive(Debug, Clone)]
pub struct VisibleGameState {
    pub game_id: i64,
    pub player_seat: i16,
    pub game_state: DbGameState,
    pub current_round: i16,
    pub hand_size: u8,
    pub dealer_pos: i16,

    /// Player's hand (cards they can play)
    pub hand: Vec<Card>,

    /// Bids placed so far this round (by seat position)
    pub bids: Vec<Option<u8>>,

    /// Trump suit (if determined)
    pub trump: Option<Trump>,

    /// Current trick number
    pub trick_no: i16,

    /// Cards played in current trick (if any)
    pub current_trick_plays: Vec<(i16, Card)>,

    /// Cumulative scores for all players
    pub scores: [i16; 4],
}

impl VisibleGameState {
    /// Load visible game state for a player from the database.
    pub async fn load<C: ConnectionTrait + Send + Sync>(
        conn: &C,
        game_id: i64,
        player_seat: i16,
    ) -> Result<Self, AppError> {
        use crate::adapters::games_sea;

        // Load game
        let game = games_sea::require_game(conn, game_id).await?;

        let current_round = game.current_round.ok_or_else(|| {
            DomainError::validation(ValidationKind::Other("NO_ROUND".into()), "No current round")
        })?;

        let hand_size = game.hand_size().ok_or_else(|| {
            DomainError::validation(ValidationKind::InvalidHandSize, "Hand size not set")
        })? as u8;

        let dealer_pos = game.dealer_pos().unwrap_or(0);

        // Load round
        let round = rounds::find_by_game_and_round(conn, game_id, current_round)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // Load player's hand
        let hand_record = hands::find_by_round_and_seat(conn, round.id, player_seat)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("HAND_NOT_FOUND".into()),
                    "Hand not found",
                )
            })?;

        let hand: Vec<Card> = hand_record
            .cards
            .iter()
            .map(|c| from_stored_format(&c.suit, &c.rank))
            .collect::<Result<Vec<_>, _>>()?;

        // Load bids
        let bid_records = bids::find_all_by_round(conn, round.id).await?;
        let mut bids = vec![None; 4];
        for bid in bid_records {
            if bid.player_seat >= 0 && bid.player_seat < 4 {
                bids[bid.player_seat as usize] = Some(bid.bid_value as u8);
            }
        }

        // Load trump
        let trump = round.trump.map(|t| match t {
            rounds::Trump::Clubs => Trump::Clubs,
            rounds::Trump::Diamonds => Trump::Diamonds,
            rounds::Trump::Hearts => Trump::Hearts,
            rounds::Trump::Spades => Trump::Spades,
            rounds::Trump::NoTrump => Trump::NoTrump,
        });

        // Load current trick plays (if in TrickPlay phase)
        let mut current_trick_plays = Vec::new();
        if game.state == DbGameState::TrickPlay {
            let trick_no = game.current_trick_no;
            if let Some(trick) = tricks::find_by_round_and_trick(conn, round.id, trick_no).await? {
                let play_records = plays::find_all_by_trick(conn, trick.id).await?;
                for play in play_records {
                    let card = from_stored_format(&play.card.suit, &play.card.rank)?;
                    current_trick_plays.push((play.player_seat, card));
                }
            }
        }

        // Load scores (stub - would load from previous round's scores)
        let scores = [0, 0, 0, 0]; // TODO: load cumulative scores

        Ok(Self {
            game_id,
            player_seat,
            game_state: game.state,
            current_round,
            hand_size,
            dealer_pos,
            hand,
            bids,
            trump,
            trick_no: game.current_trick_no,
            current_trick_plays,
            scores,
        })
    }

    /// Get legal bids for this player.
    ///
    /// Returns empty vec if not in Bidding phase or not player's turn.
    pub fn legal_bids(&self) -> Result<Vec<u8>, AppError> {
        if self.game_state != DbGameState::Bidding {
            return Ok(Vec::new());
        }

        // Check if it's this player's turn
        let bid_count = self.bids.iter().filter(|b| b.is_some()).count();
        let expected_seat = (self.dealer_pos + 1 + bid_count as i16) % 4;
        if self.player_seat != expected_seat {
            return Ok(Vec::new());
        }

        let mut legal = valid_bid_range(self.hand_size).collect::<Vec<_>>();

        // Dealer restriction: if last to bid, cannot make sum equal hand_size
        if bid_count == 3 {
            let existing_sum: u8 = self.bids.iter().filter_map(|&b| b).sum();
            let forbidden = self.hand_size.saturating_sub(existing_sum);
            legal.retain(|&b| b != forbidden);
        }

        Ok(legal)
    }

    /// Get legal plays for this player.
    ///
    /// Returns empty vec if not in TrickPlay phase or not player's turn.
    pub fn legal_plays(&self) -> Result<Vec<Card>, AppError> {
        if self.game_state != DbGameState::TrickPlay {
            return Ok(Vec::new());
        }

        // Determine whose turn it is
        let play_count = self.current_trick_plays.len();
        let leader_seat = if play_count == 0 {
            // First play of trick - determined by previous trick winner or bidding winner
            // For now, assume current turn logic handles this
            // This is a simplification - full logic would track trick leader
            self.player_seat // Placeholder
        } else {
            // Not first play - need to follow turn order
            let first_player = self.current_trick_plays[0].0;
            (first_player + play_count as i16) % 4
        };

        if self.player_seat != leader_seat && play_count > 0 {
            // Not our turn
            return Ok(Vec::new());
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

        Ok(legal)
    }

    /// Get legal trump choices (all suits + NoTrump).
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
