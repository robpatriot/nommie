use sea_orm::{DatabaseTransaction, EntityTrait};
use tracing::{debug, info};

use super::GameFlowService;
use crate::ai::{create_ai, AiConfig};
use crate::entities::ai_profiles;
use crate::entities::games::GameState as DbGameState;
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::{bids, games, memberships, player_view, plays, rounds, tricks};

/// Type of action needed from a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActionType {
    Bid,
    Trump,
    Play,
}

impl GameFlowService {
    /// Check if an AI player needs to act and execute the action (with optional cache).
    ///
    /// Returns true if an AI action was executed (which will trigger recursive processing).
    ///
    /// If round_cache is provided, uses it to avoid redundant database queries.
    /// If game_history is provided, wraps it in GameContext and passes to AI methods.
    /// Otherwise, loads player/AI data from database (slower fallback).
    pub(super) async fn check_and_execute_ai_action_with_cache(
        &self,
        txn: &DatabaseTransaction,
        game: &games::Game,
        round_cache: Option<&crate::services::round_cache::RoundCache>,
        game_history: Option<&crate::domain::player_view::GameHistory>,
    ) -> Result<bool, AppError> {
        // Determine whose turn it is
        let action_info = self.determine_next_action(txn, game).await?;

        let Some((player_seat, action_type)) = action_info else {
            return Ok(false); // No action needed
        };

        // Check if this player is an AI (use cache if available)
        let (ai_profile_id, game_player_id, profile) = if let Some(ctx) = round_cache {
            // Fast path: Use cached player data
            // If no players exist (test scenario), fall through to slow path
            if ctx.players.is_empty() {
                // No players means no AI to process (test scenario or empty game)
                debug!(game.id, "No players in game, stopping AI processing");
                return Ok(false);
            }

            let player = ctx
                .players
                .iter()
                .find(|m| m.turn_order == Some(player_seat));

            let Some(player) = player else {
                // Player not found at this seat - stop AI processing
                debug!(
                    game.id,
                    player_seat, "No player at seat, stopping AI processing"
                );
                return Ok(false);
            };

            if player.player_kind != crate::entities::game_players::PlayerKind::Ai {
                debug!(
                    game.id,
                    player_seat, "Human player's turn, stopping AI processing"
                );
                return Ok(false);
            }
            let profile_id = player.ai_profile_id.ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("AI_PROFILE_NOT_SET".into()),
                    format!("AI player at seat {player_seat} missing profile"),
                )
            })?;
            let profile = ctx.get_ai_profile(profile_id);

            if profile.is_none() {
                debug!(
                    game.id,
                    player_seat, "AI profile missing from cache, stopping AI processing"
                );
                return Ok(false);
            }

            (profile_id, player.id, profile.cloned())
        } else {
            // Slow path: Load from database (used for Lobby, Dealing, etc.)
            let memberships = memberships::find_all_by_game(txn, game.id).await?;
            let Some(player) = memberships
                .iter()
                .find(|m| m.turn_order == Some(player_seat))
            else {
                debug!(
                    game.id,
                    player_seat, "No player found at seat, stopping AI processing"
                );
                return Ok(false);
            };

            if player.player_kind != crate::entities::game_players::PlayerKind::Ai {
                debug!(
                    game.id,
                    player_seat, "Human player's turn, stopping AI processing"
                );
                return Ok(false);
            }

            let profile_id = player.ai_profile_id.ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("AI_PROFILE_NOT_FOUND".into()),
                    format!("AI profile not linked for player {}", player.id),
                )
            })?;

            let profile = ai_profiles::Entity::find_by_id(profile_id)
                .one(txn)
                .await?
                .ok_or_else(|| {
                    DomainError::validation(
                        ValidationKind::Other("AI_PROFILE_NOT_FOUND".into()),
                        format!("AI profile {profile_id} not found"),
                    )
                })?;

            (profile_id, player.id, Some(profile))
        };

        let profile = profile.ok_or_else(|| {
            AppError::internal(
                crate::errors::ErrorCode::InternalError,
                "AI profile not available",
                std::io::Error::other(format!("AI profile missing for id {ai_profile_id}")),
            )
        })?;

        info!(game.id, player_seat, action = ?action_type, "Processing AI turn");

        // Load any AI overrides for this game instance
        let ai_override =
            crate::repos::ai_overrides::find_by_game_player_id(txn, game_player_id).await?;

        // Resolve effective AI configuration (overrides take precedence over profile)
        let effective_memory_level = ai_override
            .as_ref()
            .and_then(|o| o.memory_level)
            .or(profile.memory_level)
            .unwrap_or(100);

        let effective_config = if let Some(ref override_data) = ai_override {
            if let Some(ref override_config) = override_data.config {
                // Merge configs: override fields take precedence
                crate::services::ai::merge_json_configs(
                    profile.config.as_ref(),
                    Some(override_config),
                )
            } else {
                profile.config.clone()
            }
        } else {
            profile.config.clone()
        };

        // Create AI player with effective config
        let ai_type = profile.registry_name.as_str();
        let config = AiConfig::from_json(effective_config.as_ref());
        let use_memory_recency = config.memory_recency();
        let ai = create_ai(ai_type, config).ok_or_else(|| {
            AppError::internal(
                crate::errors::ErrorCode::InternalError,
                "unknown AI type",
                std::io::Error::other(format!("AI type '{ai_type}' is not supported")),
            )
        })?;

        debug!(
            game.id,
            player_seat,
            memory_level = effective_memory_level,
            has_overrides = ai_override.is_some(),
            memory_recency = use_memory_recency,
            "AI configuration resolved"
        );

        // Execute action with retries, using cache if available
        const MAX_RETRIES_PER_ACTION: usize = 3;
        let mut last_error = None;

        for retry in 0..MAX_RETRIES_PER_ACTION {
            // Build current round info - use cache if available
            let state = if let Some(cache) = round_cache {
                // Fast path: Use cached data
                let game_model = crate::repos::games::require_game(txn, game.id).await?;
                cache
                    .build_current_round_info(
                        txn,
                        player_seat,
                        game_model.state,
                        game_model.current_trick_no,
                    )
                    .await?
            } else {
                // Fallback: Load everything from DB
                player_view::load_current_round_info(txn, game.id, player_seat).await?
            };

            // Build RoundMemory if memory is enabled
            let round_memory = if effective_memory_level > 0 {
                let current_round_no = game.current_round.ok_or_else(|| {
                    AppError::internal_msg(
                        crate::errors::ErrorCode::InternalError,
                        "current round not available",
                        "no current round when building AI memory",
                    )
                })?;
                let round = rounds::find_by_game_and_round(txn, game.id, current_round_no)
                    .await?
                    .ok_or_else(|| {
                        AppError::internal(
                            crate::errors::ErrorCode::InternalError,
                            "round data unavailable for memory",
                            std::io::Error::other(format!(
                                "round {current_round_no} not found when building AI memory"
                            )),
                        )
                    })?;

                // Load and degrade memory using the shared helper function
                let memory_mode =
                    crate::ai::MemoryMode::from_db_value(Some(effective_memory_level));

                // Derive deterministic memory seed from game seed (not AI config seed)
                // This ensures memory is stable within a round and unique per player
                let memory_seed = game.rng_seed.map(|game_seed| {
                    crate::domain::derive_memory_seed(game_seed, current_round_no, player_seat)
                });

                let degraded_tricks = crate::ai::memory::get_round_card_plays(
                    txn,
                    round.id,
                    memory_mode,
                    memory_seed,
                    use_memory_recency,
                )
                .await?;

                if !degraded_tricks.is_empty() {
                    Some(crate::domain::RoundMemory::new(
                        memory_mode,
                        degraded_tricks,
                    ))
                } else {
                    None
                }
            } else {
                None
            };

            // Create GameContext with cached game history and round memory
            let history = game_history.ok_or_else(|| {
                AppError::internal_msg(
                    crate::errors::ErrorCode::InternalError,
                    "game history not cached",
                    "GameHistory not available - should be cached by orchestration",
                )
            })?;
            let game_context = crate::domain::GameContext::new(game.id)
                .with_history(history.clone())
                .with_round_memory(round_memory);

            // Execute AI decision and persist the action
            let result = match action_type {
                ActionType::Bid => {
                    let bid = ai.choose_bid(&state, &game_context)?;
                    // Use internal version to avoid recursion (loop handles processing)
                    // Service loads its own validation data (trust boundary)
                    self.submit_bid_internal(txn, game.id, player_seat, bid, game.version)
                        .await
                }
                ActionType::Trump => {
                    let trump_choice = ai.choose_trump(&state, &game_context)?;
                    // Use internal version to avoid recursion (loop handles processing)
                    self.set_trump_internal(txn, game.id, player_seat, trump_choice, game.version)
                        .await
                }
                ActionType::Play => {
                    let card = ai.choose_play(&state, &game_context)?;
                    // Use internal version to avoid recursion (loop handles processing)
                    self.play_card_internal(txn, game.id, player_seat, card, game.version)
                        .await
                }
            };

            match result {
                Ok(_) => {
                    info!(
                        game.id,
                        player_seat,
                        action = ?action_type,
                        retry,
                        cached = round_cache.is_some(),
                        "AI action executed successfully"
                    );
                    return Ok(true);
                }
                Err(e) => {
                    tracing::warn!(
                        game.id,
                        player_seat,
                        retry,
                        error = ?e,
                        "AI action failed"
                    );
                    last_error = Some(e);
                }
            }
        }

        // All retries exhausted
        Err(last_error.unwrap_or_else(|| {
            AppError::internal_msg(
                crate::errors::ErrorCode::InternalError,
                "AI action failed with no error details",
                "all retry attempts exhausted with no error details captured",
            )
        }))
    }

    /// Determine what action is needed next and whose turn it is.
    ///
    /// Returns None if no action is needed (game complete or waiting).
    /// Returns Some((seat, action_type)) if an action is needed.
    pub(super) async fn determine_next_action(
        &self,
        txn: &DatabaseTransaction,
        game: &games::Game,
    ) -> Result<Option<(u8, ActionType)>, AppError> {
        match game.state {
            DbGameState::Lobby | DbGameState::Dealing | DbGameState::BetweenRounds => {
                // No action needed - check_and_apply_transition handles state changes
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

                let bid_count = bids::count_bids_by_round(txn, round.id).await?;
                if bid_count >= 4 {
                    // All bids placed - no action needed (state transition will happen)
                    Ok(None)
                } else {
                    let dealer_pos = game.dealer_pos().unwrap_or(0);
                    let next_seat = (dealer_pos + 1 + bid_count as u8) % 4;
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
                    // Trick exists - determine next player based on current plays
                    let play_count = plays::count_plays_by_trick(txn, trick.id).await?;
                    let all_plays = plays::find_all_by_trick(txn, trick.id).await?;
                    let first_player = all_plays.first().map(|p| p.player_seat).unwrap_or(0);
                    let next_seat = (first_player + play_count as u8) % 4;
                    Ok(Some((next_seat, ActionType::Play)))
                } else {
                    // First play of trick - need to determine leader
                    let leader = if current_trick_no == 1 {
                        // First trick: player to left of dealer leads
                        let dealer_pos = game.dealer_pos().ok_or_else(|| {
                            DomainError::validation(
                                ValidationKind::Other("NO_DEALER_POS".into()),
                                "Dealer position not set",
                            )
                        })?;
                        (dealer_pos + 1) % 4
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
            DbGameState::Scoring | DbGameState::Completed | DbGameState::Abandoned => {
                // No action needed - check_and_apply_transition handles Scoring state
                Ok(None)
            }
        }
    }
}
