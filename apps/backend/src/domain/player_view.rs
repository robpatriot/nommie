//! Player view of game state - what information is visible to a player.
//!
//! This module provides CurrentRoundInfo which represents all information
//! available to a player at their decision point for the current round, including legal moves.
//! It also provides GameHistory for accessing all public game history (bids, trumps, scores).

use sea_orm::ConnectionTrait;

use crate::domain::cards_parsing::from_stored_format;
use crate::domain::{valid_bid_range, Card, Trump};
use crate::entities::games::GameState as DbGameState;
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::{bids, hands, plays, rounds, scores, tricks};

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
/// Used by both AI players and to render UI for human players.
#[derive(Debug, Clone)]
pub struct CurrentRoundInfo {
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

    /// Player who should lead the current trick (None if not in TrickPlay)
    pub trick_leader: Option<i16>,
}

impl CurrentRoundInfo {
    /// Load current round info for a player from the database.
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

        // Load player's original dealt hand
        let hand_record = hands::find_by_round_and_seat(conn, round.id, player_seat)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("HAND_NOT_FOUND".into()),
                    "Hand not found",
                )
            })?;

        let original_hand: Vec<Card> = hand_record
            .cards
            .iter()
            .map(|c| from_stored_format(&c.suit, &c.rank))
            .collect::<Result<Vec<_>, _>>()?;

        // Load all cards this player has played this round (across all tricks)
        let all_round_tricks = tricks::find_all_by_round(conn, round.id).await?;
        let mut played_cards: Vec<Card> = Vec::new();

        for trick in all_round_tricks {
            let trick_plays = plays::find_all_by_trick(conn, trick.id).await?;
            for play in trick_plays {
                if play.player_seat == player_seat {
                    let card = from_stored_format(&play.card.suit, &play.card.rank)?;
                    played_cards.push(card);
                }
            }
        }

        // Compute remaining hand = original - played
        let mut hand = original_hand;
        for played in played_cards {
            if let Some(pos) = hand.iter().position(|c| *c == played) {
                hand.remove(pos);
            }
        }

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

        // Load cumulative scores from completed rounds
        use crate::repos::scores;
        let scores = scores::get_scores_for_completed_rounds(conn, game_id, current_round).await?;

        // Determine trick leader (who should play first)
        let trick_leader = if game.state == DbGameState::TrickPlay {
            let current_trick_no = game.current_trick_no;
            let prev_trick_winner = if current_trick_no > 0 {
                tricks::find_by_round_and_trick(conn, round.id, current_trick_no - 1)
                    .await?
                    .map(|t| t.winner_seat)
            } else {
                None
            };
            determine_trick_leader(current_trick_no, dealer_pos, prev_trick_winner)
        } else {
            None
        };

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
            trick_leader,
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

/// Complete game history including all rounds with bids, trumps, and scores.
///
/// Used for score table display and AI game history access.
#[derive(Debug, Clone)]
pub struct GameHistory {
    pub rounds: Vec<RoundHistory>,
}

/// History of a single round.
#[derive(Debug, Clone)]
pub struct RoundHistory {
    pub round_no: i16,
    pub dealer_seat: i16,
    pub bids: [Option<u8>; 4],
    pub trump_selector_seat: Option<i16>,
    pub trump: Option<Trump>,
    pub scores: [RoundScoreDetail; 4],
}

/// Score details for a player in a round.
#[derive(Debug, Clone, Copy)]
pub struct RoundScoreDetail {
    pub round_score: i16,
    pub cumulative_score: i16,
}

impl GameHistory {
    /// Load complete game history for a game.
    ///
    /// Returns all rounds (completed and partially completed current round) with their
    /// bids, trump selector, trump choice, and scores.
    ///
    /// TODO: Consider caching game history since it only changes on round completion.
    /// Could be cached in RoundContext or a separate GameHistoryCache structure.
    /// For now, this is loaded on-demand which is acceptable since:
    /// - History is only needed for score table display (infrequent)
    /// - AI players can cache it themselves if needed
    /// - Query cost is low (few rounds per game, simple joins)
    pub async fn load<C: ConnectionTrait + Send + Sync>(
        conn: &C,
        game_id: i64,
    ) -> Result<Self, AppError> {
        // Load all rounds for this game
        let all_rounds = rounds::find_all_by_game(conn, game_id).await?;

        let mut round_histories = Vec::new();

        for round in all_rounds {
            // Load bids for this round
            let bid_records = bids::find_all_by_round(conn, round.id).await?;
            let mut bids = [None; 4];
            for bid in &bid_records {
                if bid.player_seat >= 0 && bid.player_seat < 4 {
                    bids[bid.player_seat as usize] = Some(bid.bid_value as u8);
                }
            }

            // Calculate trump_selector_seat (winning bidder) from bids
            // Only calculate if all bids are present
            let trump_selector_seat = if bids.iter().all(|b| b.is_some()) {
                calculate_winning_bidder(&bids, round.dealer_pos)
            } else {
                None
            };

            // Convert trump
            let trump = round.trump.map(|t| match t {
                rounds::Trump::Clubs => Trump::Clubs,
                rounds::Trump::Diamonds => Trump::Diamonds,
                rounds::Trump::Hearts => Trump::Hearts,
                rounds::Trump::Spades => Trump::Spades,
                rounds::Trump::NoTrump => Trump::NoTrump,
            });

            // Load scores for this round (if the round is completed)
            let score_records = scores::find_all_by_round(conn, round.id).await?;
            let mut round_scores = [RoundScoreDetail {
                round_score: 0,
                cumulative_score: 0,
            }; 4];

            for score in score_records {
                if score.player_seat >= 0 && score.player_seat < 4 {
                    round_scores[score.player_seat as usize] = RoundScoreDetail {
                        round_score: score.round_score,
                        cumulative_score: score.total_score_after,
                    };
                }
            }

            round_histories.push(RoundHistory {
                round_no: round.round_no,
                dealer_seat: round.dealer_pos,
                bids,
                trump_selector_seat,
                trump,
                scores: round_scores,
            });
        }

        Ok(GameHistory {
            rounds: round_histories,
        })
    }
}

/// Calculate the winning bidder from bids.
///
/// Returns the seat of the player with the highest bid.
/// Ties are broken by earliest bidder (from dealer+1 clockwise).
fn calculate_winning_bidder(bids: &[Option<u8>; 4], dealer_pos: i16) -> Option<i16> {
    let mut best_bid: Option<u8> = None;
    let mut winner: Option<i16> = None;

    // Start from dealer+1 and go clockwise
    let start = (dealer_pos + 1) % 4;

    for i in 0..4 {
        let seat = (start + i) % 4;
        if let Some(bid_value) = bids[seat as usize] {
            match best_bid {
                None => {
                    best_bid = Some(bid_value);
                    winner = Some(seat);
                }
                Some(curr) => {
                    if bid_value > curr {
                        best_bid = Some(bid_value);
                        winner = Some(seat);
                    }
                }
            }
        }
    }

    winner
}
