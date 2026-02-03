use sea_orm::DatabaseTransaction;
use time::OffsetDateTime;
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
        let hand_size = hand_size_for_round(next_round).ok_or_else(|| {
            DomainError::validation(
                ValidationKind::Other("MAX_ROUNDS".into()),
                "All rounds complete",
            )
        })?;

        // Derive deterministic dealing seed from game seed
        // Game seed is generated from entropy at creation, then all randomness flows from it
        let game_seed = crate::domain::require_seed_32(&game.rng_seed)?;
        let dealing_seed = crate::domain::derive_dealing_seed(&game_seed, next_round)?;

        // Deal hands using domain logic
        let dealt_hands = deal_hands(4, hand_size, dealing_seed)?;

        // Update DB: state and round number
        let starting_dealer_pos = if next_round == 1 { Some(0) } else { None };
        let waiting_since = if next_round == 1 {
            Some(Some(OffsetDateTime::now_utc()))
        } else {
            None
        };
        let updated_game = games::update_game(
            txn,
            game_id,
            game.version,
            games::GameUpdatePatch {
                state: Some(DbGameState::Bidding),
                current_round: Some(next_round),
                starting_dealer_pos,
                current_trick_no: None,
                waiting_since,
            },
        )
        .await?;

        // Create round record in DB
        let round = rounds::create_round(txn, game_id, next_round).await?;

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
            hand_size = hand_size,
            dealer_pos = updated_game.dealer_pos().unwrap_or(0),
            "Round dealt successfully"
        );
        debug!(game_id, round = next_round, "Transition: -> Bidding");

        Ok(())
    }

    /// Internal version that accepts game object to avoid redundant loads.
    /// Used by production code (player_actions) and accessible to test helpers.
    /// Note: Public for test access (integration tests are separate crate).
    pub async fn score_round_internal(
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

        // Load GameState and apply domain scoring logic
        let mut state = {
            use crate::services::games::GameService;
            let service = GameService;
            service.load_game_state(txn, game_id).await?
        };
        // Force Scoring phase for this explicit scoring entrypoint. The domain
        // function is idempotent and will be a no-op if scoring was already applied.
        state.phase = crate::domain::state::Phase::Scoring;

        // Apply domain scoring logic
        let _result = crate::domain::scoring::apply_round_scoring(&mut state);

        // Load bids and tricks from the database to materialize round_scores rows.
        // Domain state (bids, tricks_won, scores_total) is the source of truth for
        // scoring; the DB rows are a persisted projection.
        let all_bids = bids::find_all_by_round(txn, round.id).await?;
        if all_bids.len() != 4 {
            return Err(DomainError::validation(
                ValidationKind::Other("INCOMPLETE_BIDS".into()),
                format!("Need 4 bids, got {}", all_bids.len()),
            )
            .into());
        }

        let all_tricks = tricks::find_all_by_round(txn, round.id).await?;
        let mut tricks_won = [0u8; 4];
        for trick in &all_tricks {
            if trick.winner_seat < 4 {
                tricks_won[trick.winner_seat as usize] += 1;
            } else {
                return Err(DomainError::validation(
                    ValidationKind::Other("INVALID_TRICK_WINNER".into()),
                    format!(
                        "Trick winner_seat {} out of range 0..3 for round {}",
                        trick.winner_seat, round.id
                    ),
                )
                .into());
            }
        }

        // Persist scores for all 4 players
        for seat in 0..4u8 {
            let bid = all_bids
                .iter()
                .find(|b| b.player_seat == seat)
                .map(|b| b.bid_value)
                .unwrap_or(0);
            let tricks = tricks_won[seat as usize];
            let bid_met = bid == tricks;

            // Scoring rules:
            // - base_score = tricks won (0..hand_size)
            // - bonus = 10 iff bid_met, else 0
            // - round_score = base_score + bonus
            let bonus = if bid_met { 10u8 } else { 0u8 };
            let base_score_u8 = tricks;
            let round_score_u8 = base_score_u8 + bonus;

            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round.id,
                    player_seat: seat,
                    bid_value: bid,
                    tricks_won: tricks,
                    bid_met,
                    base_score: base_score_u8,
                    bonus,
                    round_score: round_score_u8,
                    total_score_after: state.scores_total[seat as usize],
                },
            )
            .await?;
        }

        // Mark round as complete
        rounds::complete_round(txn, round.id).await?;

        // Determine next phase based on whether next round is valid (action-driven pattern)
        // Use domain function to check if we can continue to next round
        let is_game_complete = hand_size_for_round(current_round_no + 1).is_none();
        let next_state = if is_game_complete {
            // No more valid rounds - game over
            DbGameState::Completed
        } else {
            // More rounds to play - transition to Bidding (next round will be dealt)
            DbGameState::Bidding
        };

        let waiting_since = if is_game_complete { Some(None) } else { None };
        games::update_game(
            txn,
            game_id,
            game.version,
            games::GameUpdatePatch {
                state: Some(next_state),
                current_round: None,
                starting_dealer_pos: None,
                current_trick_no: None,
                waiting_since,
            },
        )
        .await?;

        if is_game_complete {
            info!(
                game_id,
                round = current_round_no,
                "Round scored, game completed"
            );
        } else {
            info!(
                game_id,
                round = current_round_no,
                "Round scored, transitioning to next round"
            );
            // Deal the next round immediately (action-driven pattern)
            let updated_game = games::require_game(txn, game_id).await?;
            self.deal_round_internal(txn, &updated_game).await?;
        }

        Ok(())
    }
}
