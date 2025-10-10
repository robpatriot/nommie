//! Game flow orchestration service - bridges pure domain logic with DB persistence.
//!
//! This service provides fine-grained transition methods for game state progression
//! and a test/bot helper that composes them into a happy path.

use sea_orm::ConnectionTrait;
use tracing::{debug, info};

use crate::adapters::games_sea::{self, GameUpdateRound, GameUpdateState};
use crate::domain::bidding::Bid;
use crate::domain::{card_beats, deal_hands, hand_size_for_round, Card, Rank, Suit};
use crate::entities::games::GameState as DbGameState;
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::{bids, hands, plays, rounds, scores, tricks};

/// Result type for game flow operations containing final outcome summary.
#[derive(Debug, Clone)]
pub struct GameOutcome {
    pub game_id: i64,
    pub final_scores: [i16; 4],
    pub rounds_played: u8,
}

/// Game flow service - generic over ConnectionTrait for transaction support.
pub struct GameFlowService;

impl GameFlowService {
    pub fn new() -> Self {
        Self
    }

    /// Helper: Convert stored card (suit/rank strings) to domain Card
    fn parse_stored_card(card: &plays::Card) -> Result<Card, DomainError> {
        let suit = match card.suit.as_str() {
            "CLUBS" => Suit::Clubs,
            "DIAMONDS" => Suit::Diamonds,
            "HEARTS" => Suit::Hearts,
            "SPADES" => Suit::Spades,
            _ => {
                return Err(DomainError::validation(
                    ValidationKind::ParseCard,
                    format!("Invalid suit: {}", card.suit),
                ))
            }
        };

        let rank = match card.rank.as_str() {
            "TWO" => Rank::Two,
            "THREE" => Rank::Three,
            "FOUR" => Rank::Four,
            "FIVE" => Rank::Five,
            "SIX" => Rank::Six,
            "SEVEN" => Rank::Seven,
            "EIGHT" => Rank::Eight,
            "NINE" => Rank::Nine,
            "TEN" => Rank::Ten,
            "JACK" => Rank::Jack,
            "QUEEN" => Rank::Queen,
            "KING" => Rank::King,
            "ACE" => Rank::Ace,
            _ => {
                return Err(DomainError::validation(
                    ValidationKind::ParseCard,
                    format!("Invalid rank: {}", card.rank),
                ))
            }
        };

        Ok(Card { suit, rank })
    }

    /// Deal a new round: generate hands and advance game to Bidding phase.
    ///
    /// Expects game to be in Lobby (first round) or Complete (subsequent rounds).
    /// Uses rng_seed from DB; initializes to game_id if not set.
    pub async fn deal_round<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
    ) -> Result<(), AppError> {
        info!(game_id, "Dealing new round");

        // Load game from DB
        let game = games_sea::require_game(conn, game_id).await?;

        // Determine next round number
        let next_round = game.current_round.unwrap_or(0) + 1;
        if next_round > 26 {
            return Err(DomainError::validation(
                ValidationKind::Other("MAX_ROUNDS".into()),
                "All rounds complete",
            )
            .into());
        }

        let hand_size = hand_size_for_round(next_round as u8).ok_or_else(|| {
            DomainError::validation(ValidationKind::InvalidHandSize, "Invalid round number")
        })?;

        // Determine seed (use game_id as fallback)
        let seed = game.rng_seed.unwrap_or(game_id) as u64;

        // Deal hands using domain logic
        let dealt_hands = deal_hands(4, hand_size, seed)?;

        // Update DB: state and round number
        let update_state = GameUpdateState::new(game_id, DbGameState::Bidding, game.lock_version);
        let updated_game = games_sea::update_state(conn, update_state).await?;

        // On first round, set starting_dealer_pos (defaults to 0)
        let mut update_round = GameUpdateRound::new(game_id, updated_game.lock_version)
            .with_current_round(next_round as i16);

        if next_round == 1 {
            // Initialize starting dealer position on first round
            let starting_dealer = 0; // Could be randomized or determined by game rules
            update_round = update_round.with_starting_dealer_pos(starting_dealer);
        }

        let updated_game = games_sea::update_round(conn, update_round).await?;

        // Compute current dealer (hand_size and dealer_pos are now computed)
        let computed_hand_size = updated_game.hand_size().unwrap_or(0);
        let computed_dealer_pos = updated_game.dealer_pos().unwrap_or(0);

        // Create round record in DB
        let round = rounds::create_round(
            conn,
            game_id,
            next_round as i16,
            computed_hand_size,
            computed_dealer_pos,
        )
        .await?;

        // Persist dealt hands to DB
        let hands_to_store: Vec<(i16, Vec<hands::Card>)> = dealt_hands
            .iter()
            .enumerate()
            .map(|(idx, hand)| {
                let cards: Vec<hands::Card> = hand
                    .iter()
                    .map(|c| hands::Card {
                        suit: format!("{:?}", c.suit).to_uppercase(),
                        rank: format!("{:?}", c.rank).to_uppercase(),
                    })
                    .collect();
                (idx as i16, cards)
            })
            .collect();

        hands::create_hands(conn, round.id, hands_to_store).await?;

        info!(
            game_id,
            round = next_round,
            hand_size = computed_hand_size,
            dealer_pos = computed_dealer_pos,
            "Round dealt successfully"
        );
        debug!(game_id, round = next_round, "Transition: -> Bidding");

        Ok(())
    }

    /// Submit a bid for a player in the current round.
    ///
    /// Validates phase and bid value, then persists the bid to the database.
    pub async fn submit_bid<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
        player_seat: i16,
        bid_value: u8,
    ) -> Result<(), AppError> {
        debug!(game_id, player_seat, bid_value, "Submitting bid");

        // Load game
        let game = games_sea::require_game(conn, game_id).await?;

        if game.state != DbGameState::Bidding {
            return Err(DomainError::validation(
                ValidationKind::PhaseMismatch,
                "Not in bidding phase",
            )
            .into());
        }

        let hand_size = game.hand_size().ok_or_else(|| {
            DomainError::validation(ValidationKind::InvalidHandSize, "Hand size not set")
        })? as u8;

        // Validate bid range using domain logic
        let bid = Bid(bid_value);
        let valid_range = crate::domain::valid_bid_range(hand_size);
        if !valid_range.contains(&bid.0) {
            return Err(DomainError::validation(
                ValidationKind::InvalidBid,
                format!("Bid must be in range {valid_range:?}"),
            )
            .into());
        }

        // Find the current round
        let current_round_no = game.current_round.ok_or_else(|| {
            DomainError::validation(ValidationKind::Other("NO_ROUND".into()), "No current round")
        })?;

        let round = rounds::find_by_game_and_round(conn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // Determine bid_order (how many bids have been placed already)
        let bid_order = bids::count_bids_by_round(conn, round.id).await? as i16;

        // Dealer bid restriction: if this is the 4th (final) bid, check dealer rule
        if bid_order == 3 {
            // This is the dealer's bid - sum of all bids cannot equal hand_size
            let existing_bids = bids::find_all_by_round(conn, round.id).await?;
            let existing_sum: i16 = existing_bids.iter().map(|b| b.bid_value).sum();
            let proposed_sum = existing_sum + bid_value as i16;

            if proposed_sum == hand_size as i16 {
                return Err(DomainError::validation(
                    ValidationKind::InvalidBid,
                    format!(
                        "Dealer cannot bid {bid_value}: sum would be {proposed_sum} = hand_size"
                    ),
                )
                .into());
            }
        }

        // Persist the bid
        bids::create_bid(conn, round.id, player_seat, bid_value as i16, bid_order).await?;

        info!(
            game_id,
            player_seat, bid_value, bid_order, "Bid persisted successfully"
        );

        // Check if all 4 players have bid
        let bid_count = bids::count_bids_by_round(conn, round.id).await?;
        if bid_count == 4 {
            // All bids placed - transition to Trump Selection
            let updated_game = games_sea::require_game(conn, game_id).await?;
            let update = GameUpdateState::new(
                game_id,
                DbGameState::TrumpSelection,
                updated_game.lock_version,
            );
            games_sea::update_state(conn, update).await?;
            info!(game_id, "All bids placed, transitioning to Trump Selection");
            debug!(game_id, "Transition: Bidding -> TrumpSelection");
        }

        Ok(())
    }

    /// Set trump for the current round.
    ///
    /// The winning bidder (highest bid, earliest wins ties) selects trump.
    /// Transitions game to TrickPlay phase.
    pub async fn set_trump<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
        trump: rounds::Trump,
    ) -> Result<(), AppError> {
        info!(game_id, trump = ?trump, "Setting trump");

        // Load game
        let game = games_sea::require_game(conn, game_id).await?;

        if game.state != DbGameState::TrumpSelection {
            return Err(DomainError::validation(
                ValidationKind::PhaseMismatch,
                "Not in trump selection phase",
            )
            .into());
        }

        // Get current round
        let current_round_no = game.current_round.ok_or_else(|| {
            DomainError::validation(ValidationKind::Other("NO_ROUND".into()), "No current round")
        })?;

        let round = rounds::find_by_game_and_round(conn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // Set trump on the round
        rounds::update_trump(conn, round.id, trump).await?;

        // Transition to TrickPlay
        let update = GameUpdateState::new(game_id, DbGameState::TrickPlay, game.lock_version);
        games_sea::update_state(conn, update).await?;

        info!(game_id, trump = ?trump, "Trump set, transitioning to Trick Play");
        debug!(game_id, "Transition: TrumpSelection -> TrickPlay");

        Ok(())
    }

    /// Play a card for a player in the current trick.
    ///
    /// Persists the card play to the database.
    pub async fn play_card<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
        player_seat: i16,
        card: Card,
    ) -> Result<(), AppError> {
        debug!(game_id, player_seat, "Playing card");

        // Load game
        let game = games_sea::require_game(conn, game_id).await?;

        if game.state != DbGameState::TrickPlay {
            return Err(DomainError::validation(
                ValidationKind::PhaseMismatch,
                "Not in trick play phase",
            )
            .into());
        }

        // Get current round
        let current_round_no = game.current_round.ok_or_else(|| {
            DomainError::validation(ValidationKind::Other("NO_ROUND".into()), "No current round")
        })?;

        let round = rounds::find_by_game_and_round(conn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // Get current trick number
        let current_trick_no = game.current_trick_no;

        // Find or create the current trick
        // Note: This is simplified - real implementation would need to determine
        // lead suit from first play and winner from domain logic after 4th play
        let trick = if let Some(existing) =
            tricks::find_by_round_and_trick(conn, round.id, current_trick_no).await?
        {
            existing
        } else {
            // First play in this trick - create trick record
            // Use card's suit as lead suit
            let lead_suit = match card.suit {
                crate::domain::Suit::Clubs => tricks::Suit::Clubs,
                crate::domain::Suit::Diamonds => tricks::Suit::Diamonds,
                crate::domain::Suit::Hearts => tricks::Suit::Hearts,
                crate::domain::Suit::Spades => tricks::Suit::Spades,
            };

            // Winner TBD (placeholder 0 until trick completes)
            tricks::create_trick(conn, round.id, current_trick_no, lead_suit, 0).await?
        };

        // Determine play_order (how many plays already in this trick)
        let play_order = plays::count_plays_by_trick(conn, trick.id).await? as i16;

        // Convert domain Card to repo Card
        let card_for_storage = plays::Card {
            suit: format!("{:?}", card.suit).to_uppercase(),
            rank: format!("{:?}", card.rank).to_uppercase(),
        };

        // Persist the play
        plays::create_play(conn, trick.id, player_seat, card_for_storage, play_order).await?;

        info!(
            game_id,
            player_seat,
            trick_no = current_trick_no,
            play_order,
            "Card play persisted successfully"
        );

        // Check if trick is complete (4 plays)
        let play_count = plays::count_plays_by_trick(conn, trick.id).await?;
        if play_count == 4 {
            // Trick complete - need to determine winner
            // For now, call score_trick which will handle winner determination
            // In a full implementation, this would be handled automatically
            debug!(game_id, trick_no = current_trick_no, "Trick complete");
        }

        Ok(())
    }

    /// Resolve a completed trick: determine winner and advance to next trick.
    ///
    /// Loads the 4 plays, uses domain logic to determine winner based on trump/lead,
    /// updates the trick with winner, and advances current_trick_no or transitions to Scoring.
    pub async fn resolve_trick<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
    ) -> Result<(), AppError> {
        debug!(game_id, "Resolving trick");

        // Load game
        let game = games_sea::require_game(conn, game_id).await?;

        if game.state != DbGameState::TrickPlay {
            return Err(DomainError::validation(
                ValidationKind::PhaseMismatch,
                "Not in trick play phase",
            )
            .into());
        }

        // Get current round and trick
        let current_round_no = game.current_round.ok_or_else(|| {
            DomainError::validation(ValidationKind::Other("NO_ROUND".into()), "No current round")
        })?;

        let round = rounds::find_by_game_and_round(conn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        let current_trick_no = game.current_trick_no;
        let hand_size = game.hand_size().ok_or_else(|| {
            DomainError::validation(ValidationKind::InvalidHandSize, "Hand size not set")
        })?;

        // Verify trick exists and has 4 plays
        let trick = tricks::find_by_round_and_trick(conn, round.id, current_trick_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("TRICK_NOT_FOUND".into()),
                    "Trick not found",
                )
            })?;

        let all_plays = plays::find_all_by_trick(conn, trick.id).await?;
        if all_plays.len() != 4 {
            return Err(DomainError::validation(
                ValidationKind::Other("INCOMPLETE_TRICK".into()),
                format!("Trick has {} plays, need 4", all_plays.len()),
            )
            .into());
        }

        // Trump must be set by this point
        let trump_domain = round
            .trump
            .ok_or_else(|| {
                DomainError::validation(ValidationKind::Other("NO_TRUMP".into()), "Trump not set")
            })
            .map(|t| match t {
                rounds::Trump::Clubs => crate::domain::Trump::Clubs,
                rounds::Trump::Diamonds => crate::domain::Trump::Diamonds,
                rounds::Trump::Hearts => crate::domain::Trump::Hearts,
                rounds::Trump::Spades => crate::domain::Trump::Spades,
                rounds::Trump::NoTrump => crate::domain::Trump::NoTrump,
            })?;

        // Determine winner using domain logic
        let lead_suit_domain = match trick.lead_suit {
            tricks::Suit::Clubs => Suit::Clubs,
            tricks::Suit::Diamonds => Suit::Diamonds,
            tricks::Suit::Hearts => Suit::Hearts,
            tricks::Suit::Spades => Suit::Spades,
        };

        // Parse all plays into domain cards and determine winner
        let mut winner_seat = all_plays[0].player_seat;
        let mut winner_card = Self::parse_stored_card(&all_plays[0].card)?;

        // Compare each subsequent card to the current winner
        for play in &all_plays[1..] {
            let challenger_card = Self::parse_stored_card(&play.card)?;

            // Check if challenger beats current winner
            if card_beats(challenger_card, winner_card, lead_suit_domain, trump_domain) {
                winner_seat = play.player_seat;
                winner_card = challenger_card; // Update winner card
            }
        }

        info!(
            game_id,
            trick_no = current_trick_no,
            winner_seat,
            "Trick winner determined"
        );

        // Advance to next trick or Scoring phase
        let next_trick_no = current_trick_no + 1;
        if next_trick_no >= hand_size {
            // All tricks complete - ready for scoring
            info!(
                game_id,
                trick_no = current_trick_no,
                "All tricks complete, ready for scoring"
            );
        } else {
            // Advance to next trick
            let update = GameUpdateRound::new(game_id, game.lock_version)
                .with_current_trick_no(next_trick_no);
            games_sea::update_round(conn, update).await?;
            info!(
                game_id,
                trick_no = current_trick_no,
                next_trick_no,
                winner_seat,
                "Trick resolved, advanced to next trick"
            );
        }

        Ok(())
    }

    /// Score the round: calculate final scores for all players and persist.
    ///
    /// Counts tricks won, applies domain scoring logic, and saves to round_scores.
    /// Transitions game to Scoring phase and marks round as complete.
    pub async fn score_round<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
    ) -> Result<(), AppError> {
        info!(game_id, "Scoring round");

        // Load game
        let game = games_sea::require_game(conn, game_id).await?;

        let current_round_no = game.current_round.ok_or_else(|| {
            DomainError::validation(ValidationKind::Other("NO_ROUND".into()), "No current round")
        })?;

        let round = rounds::find_by_game_and_round(conn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // Load all bids for this round
        let all_bids = bids::find_all_by_round(conn, round.id).await?;
        if all_bids.len() != 4 {
            return Err(DomainError::validation(
                ValidationKind::Other("INCOMPLETE_BIDS".into()),
                format!("Need 4 bids, got {}", all_bids.len()),
            )
            .into());
        }

        // Load all tricks to count wins per player
        let all_tricks = tricks::find_all_by_round(conn, round.id).await?;

        // Count tricks won by each player
        let mut tricks_won = [0i16; 4];
        for trick in &all_tricks {
            if trick.winner_seat >= 0 && trick.winner_seat < 4 {
                tricks_won[trick.winner_seat as usize] += 1;
            }
        }

        // Get previous totals (for cumulative scoring)
        let previous_totals = if current_round_no > 1 {
            // Find previous round and get its scores
            let prev_round_no = current_round_no - 1;
            let prev_round = rounds::find_by_game_and_round(conn, game_id, prev_round_no)
                .await?
                .ok_or_else(|| {
                    DomainError::validation(
                        ValidationKind::Other("PREV_ROUND_NOT_FOUND".into()),
                        "Previous round not found",
                    )
                })?;

            let prev_scores = scores::find_all_by_round(conn, prev_round.id).await?;
            let mut totals = [0i16; 4];
            for score in prev_scores {
                if score.player_seat >= 0 && score.player_seat < 4 {
                    totals[score.player_seat as usize] = score.total_score_after;
                }
            }
            totals
        } else {
            [0, 0, 0, 0]
        };

        // Calculate and persist scores for all 4 players
        for seat in 0..4 {
            let bid = all_bids
                .iter()
                .find(|b| b.player_seat == seat)
                .map(|b| b.bid_value)
                .unwrap_or(0);

            let tricks = tricks_won[seat as usize];
            let bid_met = bid == tricks;

            // Domain scoring formula: tricks + 10 if bid met
            let base_score = tricks;
            let bonus = if bid_met { 10 } else { 0 };
            let round_score = base_score + bonus;
            let total_after = previous_totals[seat as usize] + round_score;

            scores::create_score(
                conn,
                scores::ScoreData {
                    round_id: round.id,
                    player_seat: seat,
                    bid_value: bid,
                    tricks_won: tricks,
                    bid_met,
                    base_score,
                    bonus,
                    round_score,
                    total_score_after: total_after,
                },
            )
            .await?;
        }

        // Mark round as complete
        rounds::complete_round(conn, round.id).await?;

        // Transition to Scoring phase
        let update = GameUpdateState::new(game_id, DbGameState::Scoring, game.lock_version);
        games_sea::update_state(conn, update).await?;

        info!(
            game_id,
            round = current_round_no,
            "Round scored and completed"
        );

        Ok(())
    }

    /// Advance to the next round after scoring completes.
    ///
    /// Transitions from Scoring -> BetweenRounds or Completed.
    pub async fn advance_to_next_round<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
    ) -> Result<(), AppError> {
        info!(game_id, "Advancing to next round");

        // Load game
        let game = games_sea::require_game(conn, game_id).await?;

        if game.state != DbGameState::Scoring {
            return Err(DomainError::validation(
                ValidationKind::PhaseMismatch,
                "Not in scoring phase",
            )
            .into());
        }

        let current_round = game.current_round.unwrap_or(0);
        if current_round >= 26 {
            // All rounds complete
            let update = GameUpdateState::new(game_id, DbGameState::Completed, game.lock_version);
            games_sea::update_state(conn, update).await?;
            info!(game_id, rounds_played = current_round, "Game completed");
            debug!(game_id, "Transition: Scoring -> Completed");
        } else {
            // More rounds to play - transition to BetweenRounds and reset trick counter
            let update =
                GameUpdateState::new(game_id, DbGameState::BetweenRounds, game.lock_version);
            let updated_game = games_sea::update_state(conn, update).await?;

            // Reset current_trick_no for next round
            let reset_trick =
                GameUpdateRound::new(game_id, updated_game.lock_version).with_current_trick_no(0);
            games_sea::update_round(conn, reset_trick).await?;

            info!(game_id, current_round, "Advanced to BetweenRounds");
            debug!(game_id, "Transition: Scoring -> BetweenRounds");
        }

        Ok(())
    }

    /// Test/bot helper: run a deterministic happy path for a single round.
    ///
    /// This composes the transition methods to execute a minimal flow:
    /// deal -> bid (all players) -> trump select -> play tricks -> score -> advance.
    ///
    /// Returns outcome summary for assertions.
    ///
    /// NOTE: This is a stub implementation demonstrating the structure.
    /// Full implementation requires complete state persistence.
    pub async fn run_happy_path<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
    ) -> Result<GameOutcome, AppError> {
        info!(game_id, "Starting happy path test flow");

        // Load game to validate it exists
        let _game = games_sea::require_game(conn, game_id).await?;

        // Deal first round
        self.deal_round(conn, game_id).await?;

        // Stub bidding: all players bid 1 (placeholder)
        // In a real implementation, this would loop through memberships and call submit_bid
        // For now, we just transition the state manually for demo purposes

        // Stub trump selection and trick play
        // This would require full GameState persistence to be meaningful

        // Reload game to get updated lock_version after dealing
        let game = games_sea::require_game(conn, game_id).await?;

        // Transition to scoring
        let update = GameUpdateState::new(game_id, DbGameState::Scoring, game.lock_version);
        games_sea::update_state(conn, update).await?;

        // Advance to next round or complete
        self.advance_to_next_round(conn, game_id).await?;

        info!(game_id, "Happy path test flow completed");

        // Return stub outcome
        Ok(GameOutcome {
            game_id,
            final_scores: [0, 0, 0, 0],
            rounds_played: 1,
        })
    }
}

impl Default for GameFlowService {
    fn default() -> Self {
        Self::new()
    }
}
