use sea_orm::DatabaseTransaction;
use tracing::{debug, info};

use super::GameFlowService;
use crate::entities::games::GameState as DbGameState;
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::{bids, games, memberships, player_view, plays, rounds, tricks};

impl GameFlowService {
    /// Check if all players are ready and start the game if conditions are met.
    ///
    /// This is called after a player marks themselves ready or after an AI player is added.
    /// If all 4 players are ready, automatically deals the first round.
    pub async fn check_and_start_game_if_ready(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<(), AppError> {
        // Check if all players are ready
        let all_memberships = memberships::find_all_by_game(txn, game_id).await?;
        let all_ready = all_memberships.iter().all(|m| m.is_ready);

        if all_ready && all_memberships.len() == 4 {
            let game = games::require_game(txn, game_id).await?;

            // Only auto-start from the lobby; completed games (or other states) should not deal again.
            if game.state == DbGameState::Completed {
                info!(
                    game_id,
                    "All players ready but game already completed; skipping auto-start"
                );
                return Ok(());
            }

            if game.state != DbGameState::Lobby {
                info!(
                    game_id,
                    state = ?game.state,
                    "All players ready in non-lobby state; skipping auto-start"
                );
                return Ok(());
            }

            info!(game_id, "All players ready, starting game");
            // Deal first round
            self.deal_round(txn, game_id).await?;

            // Process game state - it loops internally until no AI actions or transitions are possible
            // It will exit early if game completes or is abandoned
            self.process_game_state(txn, game_id).await?;

            // Reload game to check final state after processing
            let game = games::require_game(txn, game_id).await?;

            // Early exit if game completed or abandoned (all AI players finished the game)
            if game.state == DbGameState::Completed || game.state == DbGameState::Abandoned {
                return Ok(());
            }

            // Check if next player to act is human (if process_game_state returned normally
            // and game isn't completed/abandoned, it means we're waiting for human input)
            let next_action = self.determine_next_action(txn, &game).await?;
            if let Some(action_tuple) = next_action {
                let player_seat = action_tuple.0;
                let memberships = memberships::find_all_by_game(txn, game_id).await?;
                if let Some(player) = memberships.iter().find(|m| m.turn_order == player_seat) {
                    if player.player_kind == crate::entities::game_players::PlayerKind::Human {
                        // Waiting for human input - done processing
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
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

        // Mark ready (automatically updates game updated_at)
        memberships::set_membership_ready(txn, membership.id, true).await?;

        info!(game_id, user_id, "Player marked ready");

        // Check if all players are ready and start game if so
        self.check_and_start_game_if_ready(txn, game_id).await?;

        Ok(())
    }

    /// Process game state after any action or transition.
    ///
    /// This is the core orchestrator that:
    /// 1. Checks if a state transition is needed and applies it
    /// 2. Checks if an AI player needs to act and executes the action
    /// 3. Loops until no more transitions or AI actions are needed
    ///
    /// This is a loop-based approach to avoid deep recursion and stack overflow.
    ///
    /// # Performance
    ///
    /// Maintains RoundCache across iterations within the same round,
    /// only reloading when the round number changes. This avoids ~1,400 redundant cache
    /// creations per game (reduces to ~26, once per round).
    ///
    /// # Return Values
    ///
    /// - `Ok(())`: Successfully processed game state. Returns when:
    ///   - Game is completed or abandoned (exits early)
    ///   - No more transitions or AI actions are possible (waiting for human input or game complete)
    /// - `Err(AppError::InternalError)`: Exceeded MAX_ITERATIONS limit (indicates a bug causing infinite loop)
    ///
    /// # Safety
    ///
    /// The function has a MAX_ITERATIONS guard (2000 iterations) to prevent infinite loops.
    /// A full 26-round game requires approximately 1,560 iterations (26 rounds × ~60 actions),
    /// so 2000 provides a ~28% safety margin.
    pub async fn process_game_state(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<(), AppError> {
        use crate::services::round_cache::RoundCache;

        // Safety limit: ~26 rounds × ~60 actions (4 bids + 1 trump + 52 plays) ≈ 1,560 iterations
        // 2000 provides ~28% safety margin. If exceeded, indicates a bug causing infinite loop.
        const MAX_ITERATIONS: usize = 2000;

        // Cache that persists across iterations within the same round
        let mut cached_round: Option<(u8, RoundCache)> = None;

        // Game history cache - loaded once and updated incrementally after round completion
        // This persists across all rounds in the game and is passed to AIs for strategic analysis
        let mut game_history: Option<crate::domain::player_view::GameHistory> = None;

        // Cache game object to avoid redundant loads - only reload after state changes
        let mut game = games::require_game(txn, game_id).await?;

        for _iteration in 0..MAX_ITERATIONS {
            // Early exit if game is already completed or abandoned
            // Abandoned games should stop processing immediately to avoid waiting
            // indefinitely for actions that will never come (e.g., waiting for human
            // player who abandoned the game)
            if game.state == DbGameState::Completed || game.state == DbGameState::Abandoned {
                return Ok(());
            }

            // Priority 1: Check if we need a state transition
            let transition_applied = self.check_and_apply_transition_internal(txn, &game).await?;
            if transition_applied {
                // Transition happened - reload game to get updated state and lock_version
                game = games::require_game(txn, game_id).await?;
                // Loop again - completion will be caught at line 138 on next iteration
                continue;
            }

            // Check if we need cache and if it needs refreshing
            let needs_cache = matches!(
                game.state,
                DbGameState::Bidding | DbGameState::TrumpSelection | DbGameState::TrickPlay
            );

            if needs_cache {
                // Load game history if not yet loaded
                if game_history.is_none() {
                    debug!(game_id, "Loading GameHistory cache");
                    game_history = Some(player_view::load_game_history(txn, game_id).await?);
                }

                if let Some(current_round) = game.current_round {
                    // Check if cache is stale (different round or doesn't exist)
                    let is_stale = cached_round
                        .as_ref()
                        .is_none_or(|(cached_round_no, _)| *cached_round_no != current_round);

                    if is_stale {
                        // Load fresh cache for new round
                        debug!(
                            game_id,
                            round_no = current_round,
                            "Creating RoundCache for new round"
                        );
                        let cache = RoundCache::load(txn, game_id, current_round).await?;
                        cached_round = Some((current_round, cache));

                        // Reload game history when round changes (history was updated with new scores)
                        debug!(
                            game_id,
                            round_no = current_round,
                            "Reloading GameHistory cache for new round"
                        );
                        game_history = Some(player_view::load_game_history(txn, game_id).await?);
                    }
                    // else: reuse existing cache (optimization!)
                }
            } else {
                // Not in a state that benefits from caching
                if cached_round.is_some() {
                    debug!(game_id, "Clearing RoundCache (exited round states)");
                    cached_round = None;
                }
            }

            // Priority 2: Check if an AI needs to act (pass cache if available)
            let ai_acted = self
                .check_and_execute_ai_action_with_cache(
                    txn,
                    &game,
                    cached_round.as_ref().map(|(_, ctx)| ctx),
                    game_history.as_ref(),
                )
                .await?;

            if ai_acted {
                // AI acted - reload game to get updated state and lock_version
                game = games::require_game(txn, game_id).await?;
                // Loop again (cache remains valid for next iteration!)
                // Note: Game can't be Completed here - completion only happens through
                // transition checks on the next iteration, which will be caught at line 138.
                // Similarly, Abandoned state will be caught at line 138 on next iteration.
                continue;
            }

            // Nothing to do - we're done
            return Ok(());
        }

        Err(AppError::internal(
            crate::errors::ErrorCode::InternalError,
            "process_game_state exceeded maximum iterations",
            std::io::Error::other(format!(
                "process_game_state exceeded max iterations {MAX_ITERATIONS}"
            )),
        ))
    }

    /// Check if current game state requires a transition and apply it.
    ///
    /// Returns true if a transition was applied.
    /// Does NOT call process_game_state - the caller loops instead.
    pub(super) async fn check_and_apply_transition_internal(
        &self,
        txn: &DatabaseTransaction,
        game: &games::Game,
    ) -> Result<bool, AppError> {
        match game.state {
            DbGameState::Bidding => {
                // Check if all 4 bids are in -> transition to TrumpSelection
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

                let bid_count = bids::count_bids_by_round(txn, round.id).await?;
                if bid_count == 4 {
                    // All bids placed - transition to Trump Selection
                    games::update_state(
                        txn,
                        game.id,
                        DbGameState::TrumpSelection,
                        game.lock_version,
                    )
                    .await?;
                    info!(game.id, "All bids placed, transitioning to Trump Selection");
                    debug!(game.id, "Transition: Bidding -> TrumpSelection");
                    return Ok(true);
                }
            }
            DbGameState::TrumpSelection => {
                // Check if trump is set -> transition to TrickPlay
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

                if round.trump.is_some() {
                    // Trump is set - transition to TrickPlay and initialize to trick 1
                    // Use the game object passed in instead of reloading
                    let updated_game = games::update_state(
                        txn,
                        game.id,
                        DbGameState::TrickPlay,
                        game.lock_version,
                    )
                    .await?;

                    // Initialize current_trick_no to 1 (first trick)
                    games::update_round(
                        txn,
                        game.id,
                        updated_game.lock_version,
                        None,
                        None,
                        Some(1),
                    )
                    .await?;

                    info!(game.id, "Trump set, transitioning to Trick Play");
                    debug!(game.id, "Transition: TrumpSelection -> TrickPlay");
                    return Ok(true);
                }
            }
            DbGameState::TrickPlay => {
                // Check if current trick is complete (4 plays) -> resolve trick
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

                if let Some(trick) =
                    tricks::find_by_round_and_trick(txn, round.id, game.current_trick_no).await?
                {
                    let play_count = plays::count_plays_by_trick(txn, trick.id).await?;
                    if play_count == 4 {
                        // Trick complete - resolve it
                        debug!(
                            game.id,
                            trick_no = game.current_trick_no,
                            "Trick complete, resolving"
                        );
                        self.resolve_trick_internal(txn, game).await?;
                        // State modified; caller's loop will continue processing
                        return Ok(true);
                    }
                }
            }
            DbGameState::Scoring => {
                // Check if round is scored (completed_at set) -> advance to next round
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

                if round.completed_at.is_some() {
                    // Round scored - advance to next round
                    self.advance_to_next_round_internal(txn, game).await?;
                    // State modified; caller's loop will continue processing
                    return Ok(true);
                } else {
                    // Need to score the round first
                    self.score_round_internal(txn, game).await?;
                    // State modified; caller's loop will continue processing
                    return Ok(true);
                }
            }
            DbGameState::BetweenRounds => {
                // Automatically deal next round
                self.deal_round_internal(txn, game).await?;
                // State modified; caller's loop will continue processing
                return Ok(true);
            }
            DbGameState::Lobby
            | DbGameState::Dealing
            | DbGameState::Completed
            | DbGameState::Abandoned => {
                // No automatic transitions
            }
        }

        Ok(false)
    }
}
