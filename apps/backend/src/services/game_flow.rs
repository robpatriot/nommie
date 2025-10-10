//! Game flow orchestration service - bridges pure domain logic with DB persistence.
//!
//! This service provides fine-grained transition methods for game state progression
//! and a test/bot helper that composes them into a happy path.

use sea_orm::ConnectionTrait;
use tracing::{debug, info};

use crate::adapters::games_sea::{self, GameUpdateRound, GameUpdateState};
use crate::domain::bidding::Bid;
use crate::domain::{deal_hands, hand_size_for_round, Card};
use crate::entities::games::GameState as DbGameState;
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};

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
        let hands = deal_hands(4, hand_size, seed)?;

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

        // NOTE: Hands are stored in memory for now; full persistence TBD
        // The domain GameState is not persisted yet, only the DB state fields
        let _ = hands; // Acknowledge we dealt them but don't persist yet

        // Compute current dealer for logging (hand_size and dealer_pos are now computed)
        let computed_hand_size = updated_game.hand_size().unwrap_or(0);
        let computed_dealer_pos = updated_game.dealer_pos().unwrap_or(0);

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
    /// This is a placeholder that validates phase and bid value but does not
    /// persist per-player bid state yet (requires additional schema).
    pub async fn submit_bid<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
        _membership_id: i64,
        bid_value: u8,
    ) -> Result<(), AppError> {
        debug!(
            game_id,
            membership_id = _membership_id,
            bid_value,
            "Submitting bid"
        );

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

        // TODO: Persist bid and check if all players have bid
        // For now, this is a no-op placeholder; full implementation requires bid storage

        debug!(
            game_id,
            membership_id = _membership_id,
            bid_value,
            "Bid validated"
        );

        Ok(())
    }

    /// Play a card for a player in the current trick.
    ///
    /// This is a placeholder that validates phase but does not persist trick state yet.
    pub async fn play_card<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
        _membership_id: i64,
        _card: Card,
    ) -> Result<(), AppError> {
        debug!(game_id, membership_id = _membership_id, "Playing card");

        // Load game
        let game = games_sea::require_game(conn, game_id).await?;

        if game.state != DbGameState::TrickPlay {
            return Err(DomainError::validation(
                ValidationKind::PhaseMismatch,
                "Not in trick play phase",
            )
            .into());
        }

        // TODO: Load full GameState, call domain_play_card, persist updated state
        // For now, this is a no-op placeholder

        Ok(())
    }

    /// Score the current trick and determine winner.
    ///
    /// This is a placeholder; requires full trick state persistence.
    pub async fn score_trick<C: ConnectionTrait + Send + Sync>(
        &self,
        conn: &C,
        game_id: i64,
    ) -> Result<(), AppError> {
        debug!(game_id, "Scoring trick");

        // Load game
        let game = games_sea::require_game(conn, game_id).await?;

        if game.state != DbGameState::TrickPlay {
            return Err(DomainError::validation(
                ValidationKind::PhaseMismatch,
                "Not in trick play phase",
            )
            .into());
        }

        // TODO: Resolve trick winner using domain logic, update tricks_won, advance state

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
            // More rounds to play
            let update =
                GameUpdateState::new(game_id, DbGameState::BetweenRounds, game.lock_version);
            games_sea::update_state(conn, update).await?;
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
