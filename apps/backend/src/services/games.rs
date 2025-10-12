//! Game state loading and construction services.

use sea_orm::DatabaseTransaction;

use crate::adapters::games_sea;
use crate::domain::cards_parsing::from_stored_format;
use crate::domain::state::{GameState, Phase, RoundState};
use crate::domain::{Card, Suit, Trump};
use crate::entities::games::GameState as DbGameState;
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::{bids, hands, plays, rounds, scores, tricks};

/// Game domain service (stateless).
pub struct GameService;

impl GameService {
    pub fn new() -> Self {
        Self
    }

    /// Load GameState from database (requires transaction for consistent snapshot).
    ///
    /// Reconstructs in-memory GameState by loading all persisted data for the current round.
    pub async fn load_game_state(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<GameState, AppError> {
        // 1. Load game record
        let game = games_sea::require_game(txn, game_id).await?;

        // 2. Check if game has started (has a current round)
        let current_round_no = match game.current_round {
            Some(round_no) => round_no,
            None => {
                // Game hasn't started yet - return empty initial state (no rounds exist)
                let phase = match game.state {
                    DbGameState::Lobby => Phase::Init,
                    DbGameState::Dealing => Phase::Init,
                    DbGameState::Bidding => Phase::Bidding,
                    DbGameState::TrumpSelection => Phase::TrumpSelect,
                    _ => Phase::Init,
                };

                return Ok(GameState {
                    phase,
                    round_no: 0,
                    hand_size: 0,
                    hands: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
                    turn_start: 0,
                    turn: 0,
                    leader: 0,
                    trick_no: 0,
                    scores_total: [0, 0, 0, 0],
                    round: RoundState::new(),
                });
            }
        };

        let hand_size = game.hand_size().ok_or_else(|| {
            DomainError::validation(ValidationKind::InvalidHandSize, "Hand size not set")
        })? as u8;

        // 3. Load round record
        let round = rounds::find_by_game_and_round(txn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // 4. Load player hands
        let all_hands = hands::find_all_by_round(txn, round.id).await?;
        let mut hands_array: [Vec<Card>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];

        for hand in all_hands {
            if hand.player_seat >= 0 && hand.player_seat < 4 {
                let domain_cards = hand
                    .cards
                    .iter()
                    .map(|c| from_stored_format(&c.suit, &c.rank))
                    .collect::<Result<Vec<_>, _>>()?;
                hands_array[hand.player_seat as usize] = domain_cards;
            }
        }

        // 5. Load bids
        let all_bids = bids::find_all_by_round(txn, round.id).await?;
        let mut bids_array = [None, None, None, None];
        for bid in &all_bids {
            if bid.player_seat >= 0 && bid.player_seat < 4 {
                bids_array[bid.player_seat as usize] = Some(bid.bid_value as u8);
            }
        }

        // 6. Determine winning bidder
        let winning_bidder = if bids_array.iter().all(|b| b.is_some()) {
            bids::find_winning_bid(txn, round.id)
                .await?
                .map(|b| b.player_seat as u8)
        } else {
            None
        };

        // 7. Load trump
        let trump = round.trump.map(|t| match t {
            rounds::Trump::Clubs => Trump::Clubs,
            rounds::Trump::Diamonds => Trump::Diamonds,
            rounds::Trump::Hearts => Trump::Hearts,
            rounds::Trump::Spades => Trump::Spades,
            rounds::Trump::NoTrump => Trump::NoTrump,
        });

        // 8. Count tricks won
        let all_tricks = tricks::find_all_by_round(txn, round.id).await?;
        let mut tricks_won = [0u8; 4];
        for trick in &all_tricks {
            if trick.winner_seat >= 0 && trick.winner_seat < 4 {
                tricks_won[trick.winner_seat as usize] += 1;
            }
        }

        // 9. Load current trick plays (if in TrickPlay)
        let current_trick_no = game.current_trick_no;
        let (trick_plays, trick_lead) = if let DbGameState::TrickPlay = game.state {
            if let Some(current_trick) =
                tricks::find_by_round_and_trick(txn, round.id, current_trick_no).await?
            {
                let all_plays = plays::find_all_by_trick(txn, current_trick.id).await?;

                let plays = all_plays
                    .iter()
                    .map(|p| {
                        let card = from_stored_format(&p.card.suit, &p.card.rank)?;
                        Ok((p.player_seat as u8, card))
                    })
                    .collect::<Result<Vec<_>, DomainError>>()?;

                let lead = match current_trick.lead_suit {
                    tricks::Suit::Clubs => Suit::Clubs,
                    tricks::Suit::Diamonds => Suit::Diamonds,
                    tricks::Suit::Hearts => Suit::Hearts,
                    tricks::Suit::Spades => Suit::Spades,
                };

                (plays, Some(lead))
            } else {
                (Vec::new(), None)
            }
        } else {
            (Vec::new(), None)
        };

        // 10. Load cumulative scores
        let scores_total = scores::get_current_totals(txn, game_id).await?;

        // 11. Convert DB phase to domain Phase
        let phase = match game.state {
            DbGameState::Lobby => Phase::Init,
            DbGameState::Dealing => Phase::Init,
            DbGameState::Bidding => Phase::Bidding,
            DbGameState::TrumpSelection => Phase::TrumpSelect,
            DbGameState::TrickPlay => Phase::Trick {
                trick_no: current_trick_no as u8 + 1,
            },
            DbGameState::Scoring => Phase::Scoring,
            DbGameState::BetweenRounds => Phase::Complete,
            DbGameState::Completed => Phase::GameOver,
            DbGameState::Abandoned => Phase::GameOver,
        };

        // 12. Determine turn_start, turn, leader
        let dealer_pos = game.dealer_pos().unwrap_or(0) as u8;
        let turn_start = dealer_pos;

        let leader = all_tricks
            .last()
            .map(|t| t.winner_seat as u8)
            .unwrap_or(turn_start);

        let turn = match phase {
            Phase::Bidding => {
                let bid_count = all_bids.len() as u8;
                (turn_start + bid_count) % 4
            }
            Phase::TrumpSelect => winning_bidder.unwrap_or(turn_start),
            Phase::Trick { .. } => {
                let plays_count = trick_plays.len() as u8;
                (leader + plays_count) % 4
            }
            _ => turn_start,
        };

        Ok(GameState {
            phase,
            round_no: current_round_no as u8,
            hand_size,
            hands: hands_array,
            turn_start,
            turn,
            leader,
            trick_no: current_trick_no as u8 + 1,
            scores_total,
            round: RoundState {
                trick_plays,
                trick_lead,
                tricks_won,
                trump,
                bids: bids_array,
                winning_bidder,
            },
        })
    }
}

impl Default for GameService {
    fn default() -> Self {
        Self::new()
    }
}
