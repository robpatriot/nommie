use sea_orm::DatabaseTransaction;
use tracing::{debug, info};

use super::GameFlowService;
use crate::domain::{deal_hands, hand_size_for_round};
use crate::entities::games::GameState as DbGameState;
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::{bids, games, hands, rounds, scores, tricks};

impl GameFlowService {
    /// Deal a new round: generate hands and advance game to Bidding phase.
    ///
    /// Expects game to be in Lobby (first round) or Complete (subsequent rounds).
    /// Uses derive_dealing_seed(game.rng_seed, round_no) for deterministic dealing.
    pub async fn deal_round(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<(), AppError> {
        let game = games::require_game(txn, game_id).await?;
        self.deal_round_internal(txn, &game).await
    }

    /// Internal version that accepts game object to avoid redundant loads.
    pub(super) async fn deal_round_internal(
        &self,
        txn: &DatabaseTransaction,
        game: &games::Game,
    ) -> Result<(), AppError> {
        let game_id = game.id;
        info!(game_id, "Dealing new round");

        // Determine next round number
        let next_round = game.current_round.unwrap_or(0) + 1;
        if next_round > 26 {
            return Err(DomainError::validation(
                ValidationKind::Other("MAX_ROUNDS".into()),
                "All rounds complete",
            )
            .into());
        }

        let hand_size = hand_size_for_round(next_round).ok_or_else(|| {
            DomainError::validation(ValidationKind::InvalidHandSize, "Invalid round number")
        })?;

        // Derive deterministic dealing seed from game seed
        // Game seed is generated from entropy at creation, then all randomness flows from it
        let game_seed = game.rng_seed.ok_or_else(|| {
            AppError::internal(
                crate::errors::ErrorCode::InternalError,
                "game missing RNG seed",
                std::io::Error::other(format!("Game {game_id} missing rng_seed field")),
            )
        })?;
        let dealing_seed = crate::domain::derive_dealing_seed(game_seed, next_round);

        // Deal hands using domain logic
        let dealt_hands = deal_hands(4, hand_size, dealing_seed)?;

        // Update DB: state and round number
        let updated_game =
            games::update_state(txn, game_id, DbGameState::Bidding, game.lock_version).await?;

        // On first round, set starting_dealer_pos (defaults to 0)
        let starting_dealer_pos = if next_round == 1 { Some(0) } else { None };
        let updated_game = games::update_round(
            txn,
            game_id,
            updated_game.lock_version,
            Some(next_round),
            starting_dealer_pos,
            None,
        )
        .await?;

        // Compute current dealer (hand_size and dealer_pos are now computed)
        let computed_hand_size = updated_game.hand_size().unwrap_or(0);
        let computed_dealer_pos = updated_game.dealer_pos().unwrap_or(0);

        // Create round record in DB
        let round = rounds::create_round(
            txn,
            game_id,
            next_round,
            computed_hand_size,
            computed_dealer_pos,
        )
        .await?;

        // Persist dealt hands to DB
        let hands_to_store: Vec<(u8, Vec<hands::Card>)> = dealt_hands
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
                (idx as u8, cards)
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

    /// Score the round: calculate final scores for all players and persist.
    ///
    /// Counts tricks won, applies domain scoring logic, and saves to round_scores.
    /// Transitions game to Scoring phase and marks round as complete.
    pub async fn score_round(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<(), AppError> {
        let game = games::require_game(txn, game_id).await?;
        self.score_round_internal(txn, &game).await
    }

    /// Internal version that accepts game object to avoid redundant loads.
    pub(super) async fn score_round_internal(
        &self,
        txn: &DatabaseTransaction,
        game: &games::Game,
    ) -> Result<(), AppError> {
        let game_id = game.id;
        info!(game_id, "Scoring round");

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
        let mut tricks_won = [0u8; 4];
        for trick in &all_tricks {
            if trick.winner_seat < 4 {
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
                if score.player_seat < 4 {
                    totals[score.player_seat as usize] = score.total_score_after;
                }
            }
            totals
        } else {
            [0, 0, 0, 0]
        };

        // Calculate and persist scores for all 4 players
        for seat in 0..4u8 {
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
            let total_after = previous_totals[seat as usize] + round_score as i16;

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
        games::update_state(txn, game_id, DbGameState::Scoring, game.lock_version).await?;

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
        let game = games::require_game(txn, game_id).await?;
        self.advance_to_next_round_internal(txn, &game).await
    }

    /// Internal version that accepts game object to avoid redundant loads.
    pub(super) async fn advance_to_next_round_internal(
        &self,
        txn: &DatabaseTransaction,
        game: &games::Game,
    ) -> Result<(), AppError> {
        let game_id = game.id;
        info!(game_id, "Advancing to next round");

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
            games::update_state(txn, game_id, DbGameState::Completed, game.lock_version).await?;
            info!(game_id, rounds_played = current_round, "Game completed");
            debug!(game_id, "Transition: Scoring -> Completed");
        } else {
            // More rounds to play - transition to BetweenRounds and reset trick counter
            let updated_game =
                games::update_state(txn, game_id, DbGameState::BetweenRounds, game.lock_version)
                    .await?;

            // Reset current_trick_no to 0 (no active trick between rounds)
            games::update_round(txn, game_id, updated_game.lock_version, None, None, Some(0))
                .await?;

            info!(game_id, current_round, "Advanced to BetweenRounds");
            debug!(game_id, "Transition: Scoring -> BetweenRounds");
        }

        Ok(())
    }
}
