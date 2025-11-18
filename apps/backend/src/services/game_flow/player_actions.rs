use sea_orm::DatabaseTransaction;
use tracing::{debug, info};

use super::GameFlowService;
use crate::adapters::games_sea::{self, GameUpdateRound, GameUpdateState};
use crate::domain::bidding::{validate_consecutive_zero_bids, Bid};
use crate::domain::cards_parsing::from_stored_format;
use crate::domain::{card_beats, Card, Suit};
use crate::entities::games;
use crate::entities::games::GameState as DbGameState;
use crate::error::AppError;
use crate::errors::domain::{ConflictKind, DomainError, ValidationKind};
use crate::repos::{bids, player_view, plays, rounds, tricks};

impl GameFlowService {
    /// Submit a bid for a player in the current round.
    ///
    /// Public method that records the bid and processes game state (transitions + AI).
    ///
    /// # Parameters
    /// - `expected_lock_version`: If provided, validates that the game's current lock_version matches
    ///   this value. If not provided, uses the lock_version from the database (backward compatibility).
    ///
    /// # Returns
    /// Returns the updated game model with the new lock_version after the mutation.
    pub async fn submit_bid(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        bid_value: u8,
        expected_lock_version: Option<i32>,
    ) -> Result<games::Model, AppError> {
        self.submit_bid_internal(txn, game_id, player_seat, bid_value, expected_lock_version)
            .await?;
        self.process_game_state(txn, game_id).await?;
        // Reload game after state processing to get final lock_version
        let final_game = games_sea::require_game(txn, game_id).await?;
        Ok(final_game)
    }

    /// Internal bid submission - just records the bid without processing.
    ///
    /// Used by AI loop to avoid recursion. Handlers should use submit_bid() instead.
    ///
    /// # Security
    ///
    /// This method loads its own validation data from the database rather than
    /// accepting pre-built context. Services are trust boundaries and must not
    /// rely on caller-provided data for security checks.
    pub(super) async fn submit_bid_internal(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        bid_value: u8,
        expected_lock_version: Option<i32>,
    ) -> Result<games::Model, AppError> {
        debug!(game_id, player_seat, bid_value, "Submitting bid");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

        // Validate lock version if provided (optimistic locking)
        if let Some(expected_version) = expected_lock_version {
            if game.lock_version != expected_version {
                return Err(DomainError::conflict(
                    ConflictKind::OptimisticLock,
                    format!(
                        "Resource was modified concurrently (expected version {}, actual version {}). Please refresh and retry.",
                        expected_version, game.lock_version
                    ),
                )
                .into());
            }
        }

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

        // Find the current round (needed for validation)
        let current_round_no = game.current_round.ok_or_else(|| {
            DomainError::validation(ValidationKind::Other("NO_ROUND".into()), "No current round")
        })?;

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

        // Validate consecutive zero bids rule (if bidding 0)
        if bid_value == 0 {
            // Load game history for validation (service owns its validation data)
            let history = player_view::load_game_history(txn, game_id).await?;
            validate_consecutive_zero_bids(&history, player_seat, current_round_no)?;
        }

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
        // Use expected_lock_version if provided, otherwise use the current lock_version from the game
        let lock_version_to_use = expected_lock_version.unwrap_or(game.lock_version);
        let lock_bump = GameUpdateRound::new(game_id, lock_version_to_use);
        let updated_game = games_sea::update_round(txn, lock_bump).await?;

        Ok(updated_game)
    }

    /// Set trump for the current round.
    ///
    /// Public method that sets trump and processes game state (transitions + AI).
    ///
    /// # Parameters
    /// - `expected_lock_version`: If provided, validates that the game's current lock_version matches
    ///   this value. If not provided, uses the lock_version from the database (backward compatibility).
    ///
    /// # Returns
    /// Returns the updated game model with the new lock_version after the mutation and state transitions.
    pub async fn set_trump(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        trump: rounds::Trump,
        expected_lock_version: Option<i32>,
    ) -> Result<games::Model, AppError> {
        self.set_trump_internal(txn, game_id, player_seat, trump, expected_lock_version)
            .await?;
        self.process_game_state(txn, game_id).await?;
        // Reload game after state processing to get final lock_version
        let final_game = games_sea::require_game(txn, game_id).await?;
        Ok(final_game)
    }

    /// Internal trump setting - just sets trump without processing.
    ///
    /// Used by AI loop to avoid recursion. Handlers should use set_trump() instead.
    pub(super) async fn set_trump_internal(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        trump: rounds::Trump,
        expected_lock_version: Option<i32>,
    ) -> Result<games::Model, AppError> {
        info!(game_id, player_seat, trump = ?trump, "Setting trump");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

        // Validate lock version if provided (optimistic locking)
        if let Some(expected_version) = expected_lock_version {
            if game.lock_version != expected_version {
                return Err(DomainError::conflict(
                    ConflictKind::OptimisticLock,
                    format!(
                        "Resource was modified concurrently (expected version {}, actual version {}). Please refresh and retry.",
                        expected_version, game.lock_version
                    ),
                )
                .into());
            }
        }

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

        info!(
            game_id,
            player_seat,
            trump = ?trump,
            "Trump set by winning bidder"
        );

        // Return the game (lock_version may be updated by state transition)
        let updated_game = games_sea::require_game(txn, game_id).await?;
        Ok(updated_game)
    }

    /// Play a card for a player in the current trick.
    ///
    /// Public method that records the card play and processes game state (transitions + AI).
    ///
    /// # Parameters
    /// - `expected_lock_version`: If provided, validates that the game's current lock_version matches
    ///   this value. If not provided, uses the lock_version from the database (backward compatibility).
    ///
    /// # Returns
    /// Returns the updated game model with the new lock_version after the mutation.
    pub async fn play_card(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        card: Card,
        expected_lock_version: Option<i32>,
    ) -> Result<games::Model, AppError> {
        self.play_card_internal(txn, game_id, player_seat, card, expected_lock_version)
            .await?;
        self.process_game_state(txn, game_id).await?;
        // Reload game after state processing to get final lock_version
        let final_game = games_sea::require_game(txn, game_id).await?;
        Ok(final_game)
    }

    /// Internal card play - just records the play without processing.
    ///
    /// Used by AI loop to avoid recursion. Handlers should use play_card() instead.
    pub(super) async fn play_card_internal(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        card: Card,
        expected_lock_version: Option<i32>,
    ) -> Result<games::Model, AppError> {
        debug!(game_id, player_seat, "Playing card");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

        // Validate lock version if provided (optimistic locking)
        if let Some(expected_version) = expected_lock_version {
            if game.lock_version != expected_version {
                return Err(DomainError::conflict(
                    ConflictKind::OptimisticLock,
                    format!(
                        "Resource was modified concurrently (expected version {}, actual version {}). Please refresh and retry.",
                        expected_version, game.lock_version
                    ),
                )
                .into());
            }
        }

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

        // SECURITY: Validate the card is in the player's remaining hand
        // This prevents cheating by playing cards they don't have or already played
        let player_state = player_view::load_current_round_info(txn, game_id, player_seat).await?;

        if !player_state.hand.contains(&card) {
            return Err(DomainError::validation(
                ValidationKind::CardNotInHand,
                format!(
                    "Card {:?} of {:?} is not in player's hand",
                    card.rank, card.suit
                ),
            )
            .into());
        }

        // Also validate it's a legal play (suit following rules)
        let legal_plays = player_state.legal_plays();
        if !legal_plays.contains(&card) {
            return Err(DomainError::validation(
                ValidationKind::MustFollowSuit,
                "Must follow suit if possible",
            )
            .into());
        }

        // Get current trick number
        let current_trick_no = game.current_trick_no;

        // Find or create the current trick
        // Lead suit is determined from the first play.
        // Winner is set to 0 initially and determined by resolve_trick() after all 4 plays.
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

            // Winner placeholder - use sentinel -1 until resolve_trick determines the winner
            tricks::create_trick(txn, round.id, current_trick_no, lead_suit, -1).await?
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

        // Bump lock_version on game to reflect card play state change
        // This ensures each card play increments the version (consistent with bid behavior)
        // Use expected_lock_version if provided, otherwise use the current lock_version from the game
        let lock_version_to_use = expected_lock_version.unwrap_or(game.lock_version);
        let lock_bump = GameUpdateRound::new(game_id, lock_version_to_use);
        let updated_game = games_sea::update_round(txn, lock_bump).await?;

        Ok(updated_game)
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

        // Update trick with winner
        tricks::update_winner(txn, trick.id, winner_seat).await?;

        // Advance to next trick or Scoring phase
        let next_trick_no = current_trick_no + 1;
        if next_trick_no > hand_size {
            // All tricks complete - transition to Scoring
            info!(
                game_id,
                trick_no = current_trick_no,
                "All tricks complete, transitioning to Scoring"
            );
            let update = GameUpdateState::new(game_id, DbGameState::Scoring, game.lock_version);
            games_sea::update_state(txn, update).await?;
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
}
