use sea_orm::DatabaseTransaction;
use tracing::{debug, info};

use super::GameFlowService;
use crate::domain::bidding::Bid;
use crate::domain::Card;
use crate::entities::games::GameState as DbGameState;
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::games::Game;
use crate::repos::{bids, games, player_view, plays, rounds, tricks};

impl GameFlowService {
    /// Submit a bid for a player in the current round.
    ///
    /// Public method that records the bid and processes game state (transitions + AI).
    ///
    /// # Parameters
    /// - `expected_version`: Validates that the game's current version matches this value.
    ///
    /// # Returns
    /// Returns the updated game model with the new version after the mutation.
    pub async fn submit_bid(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: u8,
        bid_value: u8,
        expected_version: i32,
    ) -> Result<Game, AppError> {
        self.submit_bid_internal(txn, game_id, player_seat, bid_value, expected_version)
            .await?;
        self.process_game_state(txn, game_id).await?;
        // Reload game after state processing to get final version
        let final_game = games::require_game(txn, game_id).await?;
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
        player_seat: u8,
        bid_value: u8,
        expected_version: i32,
    ) -> Result<Game, AppError> {
        debug!(game_id, player_seat, bid_value, "Submitting bid");

        // Load game to get current round and hand size for validation
        let game = games::require_game(txn, game_id).await?;

        if game.state != DbGameState::Bidding {
            return Err(DomainError::validation(
                ValidationKind::PhaseMismatch,
                "Not in bidding phase",
            )
            .into());
        }

        let hand_size = game.hand_size().ok_or_else(|| {
            DomainError::validation(ValidationKind::InvalidHandSize, "Hand size not set")
        })?;

        // Find the current round (needed for validation)
        let current_round_no = game.current_round.ok_or_else(|| {
            DomainError::validation(ValidationKind::Other("NO_ROUND".into()), "No current round")
        })?;

        // Validate bid range using domain logic
        let bid = Bid(bid_value);
        let valid_range = crate::domain::rules::valid_bid_range(hand_size);
        if !valid_range.contains(&bid.value()) {
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
            crate::domain::bidding::validate_consecutive_zero_bids(
                &history,
                player_seat,
                current_round_no,
            )?;
        }

        // Load GameState and apply domain logic
        let mut state = {
            use crate::services::games::GameService;
            let service = GameService;
            service.load_game_state(txn, game_id).await?
        };

        // Apply domain logic (includes all game rule validations)
        let result = crate::domain::bidding::place_bid(&mut state, player_seat, bid)?;
        let round = rounds::find_by_game_and_round(txn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // Determine bid_order (how many bids have been placed already)
        let bid_order = bids::count_bids_by_round(txn, round.id).await?;

        // Persist the bid
        bids::create_bid(txn, round.id, player_seat, bid_value, bid_order as u8).await?;

        info!(
            game_id,
            player_seat, bid_value, bid_order, "Bid persisted successfully"
        );

        // Update game state if phase transitioned to TrumpSelect
        let state_update = result
            .phase_transitioned
            .filter(|phase| matches!(phase, crate::domain::state::Phase::TrumpSelect))
            .map(|_| DbGameState::TrumpSelection);

        let updated_game = games::update_game(
            txn,
            game_id,
            expected_version,
            state_update,
            None,
            None,
            None,
        )
        .await?;

        Ok(updated_game)
    }

    /// Set trump for the current round.
    ///
    /// Public method that sets trump and processes game state (transitions + AI).
    ///
    /// # Parameters
    /// - `expected_version`: Validates that the game's current version matches this value.
    ///
    /// # Returns
    /// Returns the updated game model with the new version after the mutation and state transitions.
    pub async fn set_trump(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: u8,
        trump: crate::domain::Trump,
        expected_version: i32,
    ) -> Result<Game, AppError> {
        self.set_trump_internal(txn, game_id, player_seat, trump, expected_version)
            .await?;
        self.process_game_state(txn, game_id).await?;
        // Reload game after state processing to get final version
        let final_game = games::require_game(txn, game_id).await?;
        Ok(final_game)
    }

    /// Internal trump setting - just sets trump without processing.
    ///
    /// Used by AI loop to avoid recursion. Handlers should use set_trump() instead.
    pub(super) async fn set_trump_internal(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: u8,
        trump: crate::domain::Trump,
        expected_version: i32,
    ) -> Result<Game, AppError> {
        info!(game_id, player_seat, trump = ?trump, "Setting trump");

        // Load GameState and apply domain logic
        let mut state = {
            use crate::services::games::GameService;
            let service = GameService;
            service.load_game_state(txn, game_id).await?
        };

        // Apply domain logic
        let result = crate::domain::bidding::set_trump(&mut state, player_seat, trump)?;

        // Get current round for persistence
        let current_round_no = state.round_no;
        let round = rounds::find_by_game_and_round(txn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // Persist trump (conversion handled by rounds::update_trump)
        rounds::update_trump(txn, round.id, trump).await?;

        info!(
            game_id,
            player_seat,
            trump = ?trump,
            "Trump set by winning bidder"
        );

        // Update game state: set_trump always transitions to Trick phase
        let updated_game = games::update_game(
            txn,
            game_id,
            expected_version,
            Some(DbGameState::TrickPlay),
            None,
            None,
            Some(result.trick_no),
        )
        .await?;

        Ok(updated_game)
    }

    /// Play a card for a player in the current trick.
    ///
    /// Public method that records the card play and processes game state (transitions + AI).
    ///
    /// # Parameters
    /// - `expected_version`: Validates that the game's current version matches this value.
    ///
    /// # Returns
    /// Returns the updated game model with the new version after the mutation.
    pub async fn play_card(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: u8,
        card: Card,
        expected_version: i32,
    ) -> Result<Game, AppError> {
        self.play_card_internal(txn, game_id, player_seat, card, expected_version)
            .await?;
        self.process_game_state(txn, game_id).await?;
        // Reload game after state processing to get final version
        let final_game = games::require_game(txn, game_id).await?;
        Ok(final_game)
    }

    /// Internal card play - just records the play without processing.
    ///
    /// Used by AI loop to avoid recursion. Handlers should use play_card() instead.
    pub(super) async fn play_card_internal(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: u8,
        card: Card,
        expected_version: i32,
    ) -> Result<Game, AppError> {
        debug!(game_id, player_seat, "Playing card");

        // Load GameState and apply domain logic
        let mut state = {
            use crate::services::games::GameService;
            let service = GameService;
            service.load_game_state(txn, game_id).await?
        };

        // Get current round for persistence
        let current_round_no = state.round_no;
        let round = rounds::find_by_game_and_round(txn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // Track trick number before domain function call (to identify which trick to persist to)
        let trick_no_before = if let crate::domain::state::Phase::Trick { trick_no } = state.phase {
            trick_no
        } else {
            return Err(DomainError::validation(
                ValidationKind::PhaseMismatch,
                "Not in trick play phase",
            )
            .into());
        };
        let trick_plays_before = state.round.trick_plays.len();

        // Find or create the current trick
        let trick = if let Some(existing) =
            tricks::find_by_round_and_trick(txn, round.id, trick_no_before).await?
        {
            existing
        } else {
            // First play in this trick - create trick record
            // Domain function will set trick_lead to card.suit on first play
            // Winner placeholder - use sentinel 255 (u8::MAX) until trick is resolved
            tricks::create_trick(txn, round.id, trick_no_before, card.suit, u8::MAX).await?
        };

        // Apply domain logic
        let result = crate::domain::tricks::play_card(&mut state, player_seat, card)?;

        // Persist the play that was just added
        let card_for_storage = plays::Card {
            suit: format!("{:?}", card.suit).to_uppercase(),
            rank: format!("{:?}", card.rank).to_uppercase(),
        };
        let play_order = trick_plays_before as u8;
        plays::create_play(txn, trick.id, player_seat, card_for_storage, play_order).await?;

        info!(
            game_id,
            player_seat,
            trick_no = trick_no_before,
            play_order,
            "Card play persisted successfully"
        );

        // If trick was completed, persist trick winner
        if result.trick_completed {
            let winner = result.trick_winner.ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("NO_WINNER".into()),
                    "Trick completed but no winner found in result",
                )
            })?;

            tricks::update_winner(txn, trick.id, winner).await?;
            info!(
                game_id,
                trick_no = trick_no_before,
                winner,
                "Trick winner persisted"
            );
        }

        // Update game state based on phase transition
        let trick_no_update = if result.trick_no_after != trick_no_before {
            Some(result.trick_no_after)
        } else {
            None
        };

        let state_update =
            if result.phase_transitioned == Some(crate::domain::state::Phase::Scoring) {
                Some(DbGameState::Scoring)
            } else {
                None
            };

        let updated_game = games::update_game(
            txn,
            game_id,
            expected_version,
            state_update,
            None,
            None,
            trick_no_update,
        )
        .await?;

        // If phase transitioned to Scoring, score the round immediately (action-driven pattern)
        if result.phase_transitioned == Some(crate::domain::state::Phase::Scoring) {
            // Score the round, which will determine next phase (Bidding or GameOver) and deal next round if needed
            self.score_round_internal(txn, &updated_game).await?;
            // Reload game to get updated state after scoring
            let final_game = games::require_game(txn, game_id).await?;
            return Ok(final_game);
        }

        Ok(updated_game)
    }
}
