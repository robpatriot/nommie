//! Round cache for optimizing AI game processing.
//!
//! This module provides RoundCache which caches immutable round data
//! to avoid redundant database queries during AI processing.

use std::collections::HashMap;

use sea_orm::DatabaseTransaction;

use crate::domain::cards_parsing::from_stored_format;
use crate::domain::{Card, Trump};
use crate::entities::ai_profiles;
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::{bids, games as games_repo, hands, rounds};

/// Cached immutable data for a round.
///
/// This struct holds all data that doesn't change during a round,
/// loaded once and reused for all AI decisions in that round.
#[derive(Debug, Clone)]
pub struct RoundCache {
    pub game_id: i64,
    pub round_no: u8,
    pub round_id: i64,
    pub hand_size: u8,
    pub dealer_pos: u8,
    pub trump: Option<Trump>,

    /// All 4 player hands (indexed by seat 0-3)
    pub hands: [Vec<Card>; 4],

    /// All 4 bids (indexed by seat 0-3)
    pub bids: [Option<u8>; 4],

    /// Cumulative scores entering this round (indexed by seat 0-3)
    pub scores: [i16; 4],

    /// Player roster (game memberships)
    pub players: Vec<crate::repos::memberships::GameMembership>,

    /// AI profiles keyed by profile ID
    pub ai_profiles: HashMap<i64, ai_profiles::Model>,
}

impl RoundCache {
    /// Load round cache from database.
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
        round_no: u8,
    ) -> Result<Self, AppError> {
        // Load game to get hand_size and dealer_pos
        let game = games_repo::require_game(txn, game_id).await?;

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
            if hand_record.player_seat < 4 {
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
            if bid.player_seat < 4 {
                bids[bid.player_seat as usize] = Some(bid.bid_value);
            }
        }

        // Load trump (already domain type, no conversion needed)
        let trump = round.trump;

        // Load cumulative scores from completed rounds
        use crate::repos::scores;
        let scores = scores::get_scores_for_completed_rounds(txn, game_id, round_no).await?;

        // Load players (may be empty in test scenarios)
        let players = crate::repos::memberships::find_all_by_game(txn, game_id).await?;

        let ai_profiles = {
            let profile_ids: Vec<i64> = players
                .iter()
                .filter(|p| p.player_kind == crate::entities::game_players::PlayerKind::Ai)
                .filter_map(|p| p.ai_profile_id)
                .collect();

            if profile_ids.is_empty() {
                HashMap::new()
            } else {
                let models =
                    crate::repos::ai_profiles::find_batch_by_ids(txn, &profile_ids).await?;
                let mut profiles = HashMap::new();
                for profile in models {
                    profiles.insert(profile.id, profile);
                }
                profiles
            }
        };

        Ok(Self {
            game_id,
            round_no,
            round_id: round.id,
            hand_size: game.hand_size().ok_or_else(|| {
                DomainError::validation(ValidationKind::InvalidHandSize, "Hand size not set")
            })?,
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
    pub fn get_hand(&self, seat: u8) -> Result<&Vec<Card>, AppError> {
        if !(0..4).contains(&seat) {
            return Err(DomainError::validation(
                ValidationKind::Other("INVALID_SEAT".into()),
                format!("Invalid seat: {seat}"),
            )
            .into());
        }
        Ok(&self.hands[seat as usize])
    }

    /// Get AI profile for an AI membership.
    pub fn get_ai_profile(&self, ai_profile_id: i64) -> Option<&ai_profiles::Model> {
        self.ai_profiles.get(&ai_profile_id)
    }

    /// Build CurrentRoundInfo for a player using cached data.
    ///
    /// Only loads mutable state (current trick plays, played cards, bids during bidding) from database,
    /// everything else comes from the cache.
    pub async fn build_current_round_info(
        &self,
        txn: &DatabaseTransaction,
        player_seat: u8,
        game_state: crate::entities::games::GameState,
        current_trick_no: u8,
    ) -> Result<crate::domain::player_view::CurrentRoundInfo, AppError> {
        use crate::entities::games::GameState as DbGameState;
        use crate::repos::{plays, tricks};

        // Convert database state to domain phase
        let phase = games_repo::db_game_state_to_phase(&game_state, current_trick_no);

        // Load fresh bids if in Bidding phase (bids are mutable during this phase)
        // Otherwise use cached bids (immutable after bidding completes)
        let bids = if game_state == DbGameState::Bidding {
            // Bids are accumulating - load fresh from DB
            let bid_records = bids::find_all_by_round(txn, self.round_id).await?;
            let mut fresh_bids = [None; 4];
            for bid in bid_records {
                if bid.player_seat < 4 {
                    fresh_bids[bid.player_seat as usize] = Some(bid.bid_value);
                }
            }
            fresh_bids
        } else {
            // Use cached bids (bidding is complete)
            self.bids
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
            game_state: phase,
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
