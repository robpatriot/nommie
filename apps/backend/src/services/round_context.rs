//! Round context cache for optimizing AI game processing.
//!
//! This module provides RoundContext which caches immutable round data
//! to avoid redundant database queries during AI processing.

use std::collections::HashMap;

use sea_orm::DatabaseTransaction;

use crate::domain::cards_parsing::from_stored_format;
use crate::domain::{Card, Trump};
use crate::entities::ai_profiles;
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::{bids, hands, rounds};

/// Cached immutable data for a round.
///
/// This struct holds all data that doesn't change during a round,
/// loaded once and reused for all AI decisions in that round.
#[derive(Debug, Clone)]
pub struct RoundContext {
    pub game_id: i64,
    pub round_no: i16,
    pub round_id: i64,
    pub hand_size: u8,
    pub dealer_pos: i16,
    pub trump: Option<Trump>,

    /// All 4 player hands (indexed by seat 0-3)
    pub hands: [Vec<Card>; 4],

    /// All 4 bids (indexed by seat 0-3)
    pub bids: [Option<u8>; 4],

    /// Cumulative scores entering this round (indexed by seat 0-3)
    pub scores: [i16; 4],

    /// Player roster (game memberships)
    pub players: Vec<crate::repos::memberships::GameMembership>,

    /// AI profiles by user_id
    pub ai_profiles: HashMap<i64, ai_profiles::Model>,
}

impl RoundContext {
    /// Load round context from database.
    ///
    /// Performs batch queries to load all immutable data for the round:
    /// - Round metadata
    /// - All 4 player hands
    /// - All bids (if bidding complete)
    /// - Player roster
    /// - AI profiles
    pub async fn load(
        txn: &DatabaseTransaction,
        game_id: i64,
        round_no: i16,
    ) -> Result<Self, AppError> {
        use crate::adapters::games_sea;

        // Load game to get hand_size and dealer_pos
        let game = games_sea::require_game(txn, game_id).await?;

        // Load round
        let round = rounds::find_by_game_and_round(txn, game_id, round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    format!("Round {round_no} not found for game {game_id}"),
                )
            })?;

        // Load all hands for this round
        let hand_records = hands::find_all_by_round(txn, round.id).await?;
        let mut hands: [Vec<Card>; 4] = [vec![], vec![], vec![], vec![]];

        for hand_record in hand_records {
            if hand_record.player_seat >= 0 && hand_record.player_seat < 4 {
                let seat = hand_record.player_seat as usize;
                let cards: Vec<Card> = hand_record
                    .cards
                    .iter()
                    .map(|c| from_stored_format(&c.suit, &c.rank))
                    .collect::<Result<Vec<_>, _>>()?;
                hands[seat] = cards;
            }
        }

        // Load all bids for this round
        let bid_records = bids::find_all_by_round(txn, round.id).await?;
        let mut bids = [None; 4];
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

        // Load cumulative scores from completed rounds
        use crate::repos::scores;
        let scores = scores::get_scores_for_completed_rounds(txn, game_id, round_no).await?;

        // Load players (may be empty in test scenarios)
        let players = crate::repos::memberships::find_all_by_game(txn, game_id).await?;

        // Load AI profiles (batch query) - only if players exist
        let ai_profiles = if !players.is_empty() {
            let user_ids: Vec<i64> = players.iter().map(|p| p.user_id).collect();
            let ai_profile_vec =
                crate::repos::ai_profiles::find_batch_by_user_ids(txn, &user_ids).await?;

            let mut profiles = HashMap::new();
            for profile in ai_profile_vec {
                profiles.insert(profile.user_id, profile);
            }
            profiles
        } else {
            HashMap::new()
        };

        Ok(Self {
            game_id,
            round_no,
            round_id: round.id,
            hand_size: game.hand_size().ok_or_else(|| {
                DomainError::validation(ValidationKind::InvalidHandSize, "Hand size not set")
            })? as u8,
            dealer_pos: game.dealer_pos().unwrap_or(0),
            trump,
            hands,
            bids,
            scores,
            players,
            ai_profiles,
        })
    }

    /// Get player hand by seat.
    pub fn get_hand(&self, seat: i16) -> Result<&Vec<Card>, AppError> {
        if !(0..4).contains(&seat) {
            return Err(DomainError::validation(
                ValidationKind::Other("INVALID_SEAT".into()),
                format!("Invalid seat: {seat}"),
            )
            .into());
        }
        Ok(&self.hands[seat as usize])
    }

    /// Get AI profile for a user.
    pub fn get_ai_profile(&self, user_id: i64) -> Option<&ai_profiles::Model> {
        self.ai_profiles.get(&user_id)
    }

    /// Check if a player is AI.
    pub fn is_player_ai(&self, seat: i16) -> bool {
        if !(0..4).contains(&seat) {
            return false;
        }

        self.players
            .iter()
            .find(|p| p.turn_order == seat as i32)
            .and_then(|p| self.ai_profiles.get(&p.user_id))
            .is_some()
    }

    /// Build CurrentRoundInfo for a player using cached data.
    ///
    /// Only loads mutable state (current trick plays, played cards, bids during bidding) from database,
    /// everything else comes from the cache.
    pub async fn build_current_round_info(
        &self,
        txn: &DatabaseTransaction,
        player_seat: i16,
        game_state: crate::entities::games::GameState,
        current_trick_no: i16,
    ) -> Result<crate::domain::player_view::CurrentRoundInfo, AppError> {
        use crate::entities::games::GameState as DbGameState;
        use crate::repos::{plays, tricks};

        // Load fresh bids if in Bidding phase (bids are mutable during this phase)
        // Otherwise use cached bids (immutable after bidding completes)
        let bids = if game_state == DbGameState::Bidding {
            // Bids are accumulating - load fresh from DB
            let bid_records = bids::find_all_by_round(txn, self.round_id).await?;
            let mut fresh_bids = [None; 4];
            for bid in bid_records {
                if bid.player_seat >= 0 && bid.player_seat < 4 {
                    fresh_bids[bid.player_seat as usize] = Some(bid.bid_value as u8);
                }
            }
            fresh_bids.to_vec()
        } else {
            // Use cached bids (bidding is complete)
            self.bids.to_vec()
        };

        // Load all tricks for this round to compute remaining cards
        let all_round_tricks = tricks::find_all_by_round(txn, self.round_id).await?;

        // Load all cards this player has played this round
        let mut played_cards: Vec<Card> = Vec::new();
        let mut current_trick_plays = Vec::new();

        for trick in &all_round_tricks {
            let trick_plays = plays::find_all_by_trick(txn, trick.id).await?;

            for play in trick_plays {
                let card = from_stored_format(&play.card.suit, &play.card.rank)?;

                // Track this player's played cards
                if play.player_seat == player_seat {
                    played_cards.push(card);
                }

                // Also collect current trick plays if this is the active trick
                if game_state == DbGameState::TrickPlay && trick.trick_no == current_trick_no {
                    current_trick_plays.push((play.player_seat, card));
                }
            }
        }

        // Compute remaining hand = original (from cache) - played
        let original_hand = self.get_hand(player_seat)?;
        let mut hand = original_hand.clone();
        for played in played_cards {
            if let Some(pos) = hand.iter().position(|c| *c == played) {
                hand.remove(pos);
            }
        }

        // Determine trick leader (who should play first)
        let trick_leader = if game_state == DbGameState::TrickPlay {
            let prev_trick_winner = if current_trick_no > 0 {
                all_round_tricks
                    .iter()
                    .find(|t| t.trick_no == current_trick_no - 1)
                    .map(|t| t.winner_seat)
            } else {
                None
            };
            crate::domain::player_view::determine_trick_leader(
                current_trick_no,
                self.dealer_pos,
                prev_trick_winner,
            )
        } else {
            None
        };

        // Build CurrentRoundInfo using cached data + computed remaining hand
        Ok(crate::domain::player_view::CurrentRoundInfo {
            game_id: self.game_id,
            player_seat,
            game_state,
            current_round: self.round_no,
            hand_size: self.hand_size,
            dealer_pos: self.dealer_pos,
            hand, // ← Remaining cards (original - played) ✅
            bids, // ← Fresh during Bidding, cached otherwise ✅
            trump: self.trump,
            trick_no: current_trick_no,
            current_trick_plays,
            scores: self.scores,
            trick_leader,
        })
    }
}
