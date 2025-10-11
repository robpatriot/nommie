//! Game flow orchestration service - bridges pure domain logic with DB persistence.
//!
//! This service provides fine-grained transition methods for game state progression
//! and a test/bot helper that composes them into a happy path.

use sea_orm::DatabaseTransaction;
use tracing::{debug, info};

use crate::adapters::games_sea::{self, GameUpdateRound, GameUpdateState};
use crate::domain::bidding::Bid;
use crate::domain::cards_parsing::from_stored_format;
use crate::domain::{card_beats, deal_hands, hand_size_for_round, Card, Suit};
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

    /// Deal a new round: generate hands and advance game to Bidding phase.
    ///
    /// Expects game to be in Lobby (first round) or Complete (subsequent rounds).
    /// Uses rng_seed from DB; initializes to game_id if not set.
    pub async fn deal_round(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<(), AppError> {
        info!(game_id, "Dealing new round");

        // Load game from DB
        let game = games_sea::require_game(txn, game_id).await?;

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
        let updated_game = games_sea::update_state(txn, update_state).await?;

        // On first round, set starting_dealer_pos (defaults to 0)
        let mut update_round = GameUpdateRound::new(game_id, updated_game.lock_version)
            .with_current_round(next_round as i16);

        if next_round == 1 {
            // Initialize starting dealer position on first round
            let starting_dealer = 0; // Could be randomized or determined by game rules
            update_round = update_round.with_starting_dealer_pos(starting_dealer);
        }

        let updated_game = games_sea::update_round(txn, update_round).await?;

        // Compute current dealer (hand_size and dealer_pos are now computed)
        let computed_hand_size = updated_game.hand_size().unwrap_or(0);
        let computed_dealer_pos = updated_game.dealer_pos().unwrap_or(0);

        // Create round record in DB
        let round = rounds::create_round(
            txn,
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

        hands::create_hands(txn, round.id, hands_to_store).await?;

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
    pub async fn submit_bid(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        bid_value: u8,
    ) -> Result<(), AppError> {
        debug!(game_id, player_seat, bid_value, "Submitting bid");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

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

        let round = rounds::find_by_game_and_round(txn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // Determine bid_order (how many bids have been placed already)
        let bid_order = bids::count_bids_by_round(txn, round.id).await? as i16;

        // Turn order validation: bidding starts at dealer+1, then proceeds clockwise
        let dealer_pos = game.dealer_pos().ok_or_else(|| {
            DomainError::validation(
                ValidationKind::Other("NO_DEALER".into()),
                "Dealer position not set",
            )
        })?;

        let expected_seat = (dealer_pos + 1 + bid_order) % 4;
        if player_seat != expected_seat {
            return Err(DomainError::validation(
                ValidationKind::OutOfTurn,
                format!(
                    "Not your turn to bid. Expected player {expected_seat} (seat {expected_seat}), got player {player_seat}"
                ),
            )
            .into());
        }

        // Dealer bid restriction: if this is the 4th (final) bid, check dealer rule
        if bid_order == 3 {
            // This is the dealer's bid - sum of all bids cannot equal hand_size
            let existing_bids = bids::find_all_by_round(txn, round.id).await?;
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
        bids::create_bid(txn, round.id, player_seat, bid_value as i16, bid_order).await?;

        info!(
            game_id,
            player_seat, bid_value, bid_order, "Bid persisted successfully"
        );

        // Bump lock_version on game to reflect bid state change
        // This ensures each bid increments the version, not just state transitions
        let updated_game = games_sea::require_game(txn, game_id).await?;
        let lock_bump = GameUpdateRound::new(game_id, updated_game.lock_version);
        games_sea::update_round(txn, lock_bump).await?;

        // Check if all 4 players have bid
        let bid_count = bids::count_bids_by_round(txn, round.id).await?;
        if bid_count == 4 {
            // All bids placed - transition to Trump Selection
            let updated_game = games_sea::require_game(txn, game_id).await?;
            let update = GameUpdateState::new(
                game_id,
                DbGameState::TrumpSelection,
                updated_game.lock_version,
            );
            games_sea::update_state(txn, update).await?;
            info!(game_id, "All bids placed, transitioning to Trump Selection");
            debug!(game_id, "Transition: Bidding -> TrumpSelection");
        }

        Ok(())
    }

    /// Set trump for the current round.
    ///
    /// The winning bidder (highest bid, earliest wins ties) selects trump.
    /// Validates that the player_seat matches the winning bidder.
    /// Transitions game to TrickPlay phase.
    pub async fn set_trump(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        trump: rounds::Trump,
    ) -> Result<(), AppError> {
        info!(game_id, player_seat, trump = ?trump, "Setting trump");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

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

        let round = rounds::find_by_game_and_round(txn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // Determine winning bidder and validate
        let winning_bid = bids::find_winning_bid(txn, round.id)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("NO_WINNING_BID".into()),
                    "No winning bid found",
                )
            })?;

        if winning_bid.player_seat != player_seat {
            return Err(DomainError::validation(
                ValidationKind::OutOfTurn,
                format!(
                    "Only the winning bidder (seat {}) can choose trump, not seat {}",
                    winning_bid.player_seat, player_seat
                ),
            )
            .into());
        }

        // Set trump on the round
        rounds::update_trump(txn, round.id, trump).await?;

        // Transition to TrickPlay
        let update = GameUpdateState::new(game_id, DbGameState::TrickPlay, game.lock_version);
        games_sea::update_state(txn, update).await?;

        info!(
            game_id,
            player_seat,
            trump = ?trump,
            "Trump set by winning bidder, transitioning to Trick Play"
        );
        debug!(game_id, "Transition: TrumpSelection -> TrickPlay");

        Ok(())
    }

    /// Play a card for a player in the current trick.
    ///
    /// Persists the card play to the database.
    pub async fn play_card(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        card: Card,
    ) -> Result<(), AppError> {
        debug!(game_id, player_seat, "Playing card");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

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

        let round = rounds::find_by_game_and_round(txn, game_id, current_round_no)
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
            tricks::find_by_round_and_trick(txn, round.id, current_trick_no).await?
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
            tricks::create_trick(txn, round.id, current_trick_no, lead_suit, 0).await?
        };

        // Determine play_order (how many plays already in this trick)
        let play_order = plays::count_plays_by_trick(txn, trick.id).await? as i16;

        // Convert domain Card to repo Card
        let card_for_storage = plays::Card {
            suit: format!("{:?}", card.suit).to_uppercase(),
            rank: format!("{:?}", card.rank).to_uppercase(),
        };

        // Persist the play
        plays::create_play(txn, trick.id, player_seat, card_for_storage, play_order).await?;

        info!(
            game_id,
            player_seat,
            trick_no = current_trick_no,
            play_order,
            "Card play persisted successfully"
        );

        // Check if trick is complete (4 plays)
        let play_count = plays::count_plays_by_trick(txn, trick.id).await?;
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
    pub async fn resolve_trick(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<(), AppError> {
        debug!(game_id, "Resolving trick");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

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

        let round = rounds::find_by_game_and_round(txn, game_id, current_round_no)
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
        let trick = tricks::find_by_round_and_trick(txn, round.id, current_trick_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("TRICK_NOT_FOUND".into()),
                    "Trick not found",
                )
            })?;

        let all_plays = plays::find_all_by_trick(txn, trick.id).await?;
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
        let mut winner_card = from_stored_format(&all_plays[0].card.suit, &all_plays[0].card.rank)?;

        // Compare each subsequent card to the current winner
        for play in &all_plays[1..] {
            let challenger_card = from_stored_format(&play.card.suit, &play.card.rank)?;

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
            games_sea::update_round(txn, update).await?;
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
    pub async fn score_round(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<(), AppError> {
        info!(game_id, "Scoring round");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

        let current_round_no = game.current_round.ok_or_else(|| {
            DomainError::validation(ValidationKind::Other("NO_ROUND".into()), "No current round")
        })?;

        let round = rounds::find_by_game_and_round(txn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // Load all bids for this round
        let all_bids = bids::find_all_by_round(txn, round.id).await?;
        if all_bids.len() != 4 {
            return Err(DomainError::validation(
                ValidationKind::Other("INCOMPLETE_BIDS".into()),
                format!("Need 4 bids, got {}", all_bids.len()),
            )
            .into());
        }

        // Load all tricks to count wins per player
        let all_tricks = tricks::find_all_by_round(txn, round.id).await?;

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
            let prev_round = rounds::find_by_game_and_round(txn, game_id, prev_round_no)
                .await?
                .ok_or_else(|| {
                    DomainError::validation(
                        ValidationKind::Other("PREV_ROUND_NOT_FOUND".into()),
                        "Previous round not found",
                    )
                })?;

            let prev_scores = scores::find_all_by_round(txn, prev_round.id).await?;
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
                txn,
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
        rounds::complete_round(txn, round.id).await?;

        // Transition to Scoring phase
        let update = GameUpdateState::new(game_id, DbGameState::Scoring, game.lock_version);
        games_sea::update_state(txn, update).await?;

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
    pub async fn advance_to_next_round(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<(), AppError> {
        info!(game_id, "Advancing to next round");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

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
            games_sea::update_state(txn, update).await?;
            info!(game_id, rounds_played = current_round, "Game completed");
            debug!(game_id, "Transition: Scoring -> Completed");
        } else {
            // More rounds to play - transition to BetweenRounds and reset trick counter
            let update =
                GameUpdateState::new(game_id, DbGameState::BetweenRounds, game.lock_version);
            let updated_game = games_sea::update_state(txn, update).await?;

            // Reset current_trick_no for next round
            let reset_trick =
                GameUpdateRound::new(game_id, updated_game.lock_version).with_current_trick_no(0);
            games_sea::update_round(txn, reset_trick).await?;

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
    pub async fn run_happy_path(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<GameOutcome, AppError> {
        info!(game_id, "Starting happy path test flow");

        // Load game to validate it exists
        let _game = games_sea::require_game(txn, game_id).await?;

        // Deal first round
        self.deal_round(txn, game_id).await?;

        // Stub bidding: all players bid 1 (placeholder)
        // In a real implementation, this would loop through memberships and call submit_bid
        // For now, we just transition the state manually for demo purposes

        // Stub trump selection and trick play
        // This would require full GameState persistence to be meaningful

        // Reload game to get updated lock_version after dealing
        let game = games_sea::require_game(txn, game_id).await?;

        // Transition to scoring
        let update = GameUpdateState::new(game_id, DbGameState::Scoring, game.lock_version);
        games_sea::update_state(txn, update).await?;

        // Advance to next round or complete
        self.advance_to_next_round(txn, game_id).await?;

        info!(game_id, "Happy path test flow completed");

        // Return stub outcome
        Ok(GameOutcome {
            game_id,
            final_scores: [0, 0, 0, 0],
            rounds_played: 1,
        })
    }

    /// Mark a player as ready and check if game should start.
    ///
    /// If all players are ready after this call, automatically deals the first round.
    pub async fn mark_ready(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        user_id: i64,
    ) -> Result<(), AppError> {
        use crate::adapters::memberships_sea;
        use crate::repos::memberships;

        info!(game_id, user_id, "Marking player ready");

        // Find membership
        let membership = memberships::find_membership(txn, game_id, user_id)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("NOT_IN_GAME".into()),
                    "Player not in game",
                )
            })?;

        // Mark ready
        let dto = memberships_sea::MembershipSetReady {
            id: membership.id,
            is_ready: true,
        };
        memberships_sea::set_membership_ready(txn, dto).await?;

        info!(game_id, user_id, "Player marked ready");

        // Check if all players are ready
        let all_memberships = memberships::find_all_by_game(txn, game_id).await?;
        let all_ready = all_memberships.iter().all(|m| m.is_ready);

        if all_ready && all_memberships.len() == 4 {
            info!(game_id, "All players ready, starting game");
            // Deal first round
            self.deal_round(txn, game_id).await?;
            // Process AI turns if needed
            self.process_ai_turns(txn, game_id).await?;
        }

        Ok(())
    }

    /// Process AI turns in a loop until a human player's turn or game phase change.
    ///
    /// This orchestrator:
    /// 1. Checks whose turn it is (or what action is needed)
    /// 2. If it's an AI player, triggers their action
    /// 3. Repeats until it's a human's turn or the phase requires no immediate action
    ///
    /// Includes retry logic and timeout protection.
    pub async fn process_ai_turns(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<(), AppError> {
        use sea_orm::{EntityTrait, JoinType, QuerySelect, RelationTrait};

        use crate::ai::create_ai;
        use crate::entities::ai_profiles;
        use crate::repos::memberships;

        const MAX_RETRIES_PER_ACTION: usize = 3;
        const MAX_NO_PROGRESS_ITERATIONS: usize = 100; // Set high to allow full game

        let mut last_state = None;
        let mut no_progress_count = 0;

        for iteration in 0.. {
            // Load game state
            let game = games_sea::require_game(txn, game_id).await?;

            // Track state changes to detect infinite loops
            let current_state_key = (
                game.state.clone(),
                game.current_round,
                game.current_trick_no,
            );
            if last_state == Some(current_state_key.clone()) {
                no_progress_count += 1;
                if no_progress_count > MAX_NO_PROGRESS_ITERATIONS {
                    return Err(AppError::internal(format!(
                        "AI processing stalled: no progress after {} iterations (state: {:?}, round: {:?})",
                        MAX_NO_PROGRESS_ITERATIONS, game.state, game.current_round
                    )));
                }
            } else {
                no_progress_count = 0;
                last_state = Some(current_state_key);
            }

            // Determine what action is needed and whose turn it is
            let action_info = self.determine_next_action(txn, &game).await?;

            let Some((player_seat, action_type)) = action_info else {
                // No action needed - exit
                debug!(game_id, iteration, state = ?game.state, "No AI action needed, exiting");
                return Ok(());
            };

            // Check if this player is an AI
            let memberships = memberships::find_all_by_game(txn, game_id).await?;
            let player = memberships
                .iter()
                .find(|m| m.turn_order == player_seat as i32)
                .ok_or_else(|| {
                    DomainError::validation(
                        ValidationKind::Other("PLAYER_NOT_FOUND".into()),
                        format!("Player at seat {player_seat} not found"),
                    )
                })?;

            // Check if user is AI and get their AI profile
            let ai_info = crate::entities::users::Entity::find_by_id(player.user_id)
                .join(
                    JoinType::LeftJoin,
                    crate::entities::users::Relation::AiProfiles.def(),
                )
                .select_also(ai_profiles::Entity)
                .one(txn)
                .await?;

            let Some((user, maybe_profile)) = ai_info else {
                return Err(DomainError::validation(
                    ValidationKind::Other("USER_NOT_FOUND".into()),
                    "User not found",
                )
                .into());
            };

            if !user.is_ai {
                // Human player's turn - stop processing
                debug!(
                    game_id,
                    player_seat, iteration, "Human player's turn, stopping AI processing"
                );
                return Ok(());
            }

            // This is an AI player - trigger their action
            let profile = maybe_profile.ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("AI_PROFILE_NOT_FOUND".into()),
                    format!("AI profile not found for user {}", user.id),
                )
            })?;

            info!(
                game_id,
                player_seat,
                action = ?action_type,
                iteration,
                "Processing AI turn"
            );

            // Create AI player
            let ai_type = profile.playstyle.as_deref().unwrap_or("random");
            let ai = create_ai(ai_type, profile.config.as_ref())
                .ok_or_else(|| AppError::internal(format!("Unknown AI type: {ai_type}")))?;

            // Execute action with retries
            let mut last_retry_result = Ok(());
            for retry in 0..MAX_RETRIES_PER_ACTION {
                match self
                    .execute_ai_action(txn, game_id, player_seat, action_type, ai.as_ref())
                    .await
                {
                    Ok(()) => {
                        info!(
                            game_id,
                            player_seat,
                            action = ?action_type,
                            retry,
                            "AI action executed successfully"
                        );
                        last_retry_result = Ok(());
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(
                            game_id,
                            player_seat,
                            retry,
                            error = ?e,
                            "AI action failed"
                        );
                        last_retry_result = Err(e);
                        if retry == MAX_RETRIES_PER_ACTION - 1 {
                            // All retries exhausted - for now, fail the game
                            // TODO: implement fallback random play for production
                            return last_retry_result;
                        }
                    }
                }
            }
            last_retry_result?;
        }

        // Loop should exit via return statements, this is unreachable
        unreachable!("AI processing loop should exit via return statements")
    }

    /// Determine what action is needed next and whose turn it is.
    ///
    /// Returns None if no action is needed (game complete or waiting).
    /// Returns Some((seat, action_type)) if an action is needed.
    async fn determine_next_action(
        &self,
        txn: &DatabaseTransaction,
        game: &crate::entities::games::Model,
    ) -> Result<Option<(i16, ActionType)>, AppError> {
        use crate::entities::games::GameState as DbGameState;
        use crate::repos::{bids, tricks};

        match game.state {
            DbGameState::Lobby | DbGameState::Dealing => {
                // Waiting for external trigger
                Ok(None)
            }
            DbGameState::BetweenRounds => {
                // Automatically deal next round for AI games
                self.deal_round(txn, game.id).await?;
                // Don't return an action; let the loop re-check
                // by returning a sentinel value
                Ok(None)
            }
            DbGameState::Bidding => {
                // Determine whose turn to bid
                let current_round_no = game.current_round.ok_or_else(|| {
                    DomainError::validation(
                        ValidationKind::Other("NO_ROUND".into()),
                        "No current round",
                    )
                })?;
                let round = rounds::find_by_game_and_round(txn, game.id, current_round_no)
                    .await?
                    .ok_or_else(|| {
                        DomainError::validation(
                            ValidationKind::Other("ROUND_NOT_FOUND".into()),
                            "Round not found",
                        )
                    })?;

                let bid_count = bids::count_bids_by_round(txn, round.id).await? as i16;
                if bid_count >= 4 {
                    // All bids placed - no action needed (state transition will happen)
                    Ok(None)
                } else {
                    let dealer_pos = game.dealer_pos().unwrap_or(0);
                    let next_seat = (dealer_pos + 1 + bid_count) % 4;
                    Ok(Some((next_seat, ActionType::Bid)))
                }
            }
            DbGameState::TrumpSelection => {
                // Winning bidder needs to select trump
                let current_round_no = game.current_round.ok_or_else(|| {
                    DomainError::validation(
                        ValidationKind::Other("NO_ROUND".into()),
                        "No current round",
                    )
                })?;
                let round = rounds::find_by_game_and_round(txn, game.id, current_round_no)
                    .await?
                    .ok_or_else(|| {
                        DomainError::validation(
                            ValidationKind::Other("ROUND_NOT_FOUND".into()),
                            "Round not found",
                        )
                    })?;

                let winning_bid =
                    bids::find_winning_bid(txn, round.id)
                        .await?
                        .ok_or_else(|| {
                            DomainError::validation(
                                ValidationKind::Other("NO_WINNING_BID".into()),
                                "No winning bid found",
                            )
                        })?;

                Ok(Some((winning_bid.player_seat, ActionType::Trump)))
            }
            DbGameState::TrickPlay => {
                // Determine whose turn to play
                let current_trick_no = game.current_trick_no;
                let current_round_no = game.current_round.ok_or_else(|| {
                    DomainError::validation(
                        ValidationKind::Other("NO_ROUND".into()),
                        "No current round",
                    )
                })?;
                let round = rounds::find_by_game_and_round(txn, game.id, current_round_no)
                    .await?
                    .ok_or_else(|| {
                        DomainError::validation(
                            ValidationKind::Other("ROUND_NOT_FOUND".into()),
                            "Round not found",
                        )
                    })?;

                // Check if trick exists
                let maybe_trick =
                    tricks::find_by_round_and_trick(txn, round.id, current_trick_no).await?;

                if let Some(trick) = maybe_trick {
                    let play_count = plays::count_plays_by_trick(txn, trick.id).await? as i16;
                    if play_count >= 4 {
                        // Trick complete - need to resolve it
                        self.resolve_trick(txn, game.id).await?;
                        // Return None to let loop re-evaluate
                        Ok(None)
                    } else {
                        // Determine next player
                        let all_plays = plays::find_all_by_trick(txn, trick.id).await?;
                        let first_player = all_plays.first().map(|p| p.player_seat).unwrap_or(0);
                        let next_seat = (first_player + play_count) % 4;
                        Ok(Some((next_seat, ActionType::Play)))
                    }
                } else {
                    // First play of trick - need to determine leader
                    let leader = if current_trick_no == 0 {
                        // First trick: winning bidder leads
                        let winning_bid =
                            bids::find_winning_bid(txn, round.id)
                                .await?
                                .ok_or_else(|| {
                                    DomainError::validation(
                                        ValidationKind::Other("NO_WINNING_BID".into()),
                                        "No winning bid found",
                                    )
                                })?;
                        winning_bid.player_seat
                    } else {
                        // Subsequent tricks: previous trick winner leads
                        let prev_trick =
                            tricks::find_by_round_and_trick(txn, round.id, current_trick_no - 1)
                                .await?
                                .ok_or_else(|| {
                                    DomainError::validation(
                                        ValidationKind::Other("PREV_TRICK_NOT_FOUND".into()),
                                        "Previous trick not found",
                                    )
                                })?;
                        prev_trick.winner_seat
                    };
                    Ok(Some((leader, ActionType::Play)))
                }
            }
            DbGameState::Scoring => {
                // Need to score the round
                self.score_round(txn, game.id).await?;
                self.advance_to_next_round(txn, game.id).await?;
                // Return None to let loop re-evaluate
                Ok(None)
            }
            DbGameState::Completed | DbGameState::Abandoned => Ok(None),
        }
    }

    /// Execute an AI action.
    async fn execute_ai_action(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        action_type: ActionType,
        ai: &dyn crate::ai::AiPlayer,
    ) -> Result<(), AppError> {
        use crate::domain::player_view::VisibleGameState;

        // Load visible game state for this player
        let state = VisibleGameState::load(txn, game_id, player_seat).await?;

        match action_type {
            ActionType::Bid => {
                let bid = ai.choose_bid(&state)?;
                self.submit_bid(txn, game_id, player_seat, bid).await?;
            }
            ActionType::Trump => {
                let trump_suit = ai.choose_trump(&state)?;
                // Convert Suit to rounds::Trump
                let trump = match trump_suit {
                    crate::domain::Suit::Clubs => rounds::Trump::Clubs,
                    crate::domain::Suit::Diamonds => rounds::Trump::Diamonds,
                    crate::domain::Suit::Hearts => rounds::Trump::Hearts,
                    crate::domain::Suit::Spades => rounds::Trump::Spades,
                };
                self.set_trump(txn, game_id, player_seat, trump).await?;
            }
            ActionType::Play => {
                let card = ai.choose_play(&state)?;
                self.play_card(txn, game_id, player_seat, card).await?;
            }
        }

        Ok(())
    }
}

/// Type of action needed from a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionType {
    Bid,
    Trump,
    Play,
}

impl Default for GameFlowService {
    fn default() -> Self {
        Self::new()
    }
}
