use sea_orm::DatabaseTransaction;

use super::GameFlowService;
use crate::entities::games::GameState as DbGameState;
use crate::error::AppError;
use crate::repos::memberships::GameRole;
use crate::repos::players::{friendly_ai_name, resolve_display_name_for_membership};
use crate::repos::{ai_overrides, ai_profiles, games as games_repo, memberships};
use crate::services::game_flow::GameFlowMutationResult;
use crate::services::games::GameService;

/// Parameters for managing an AI seat's configuration.
/// Used for both adding and updating seats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManageAiSeatParams {
    pub game_id: i64,
    pub user_id: i64,
    pub seat: Option<u8>,
    pub registry_name: Option<String>,
    pub registry_version: Option<String>,
    pub config_seed: Option<u64>,
    pub expected_version: i32,
}

impl GameFlowService {
    /// Wrapper: join_game (no drain)
    pub async fn join_game(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        user_id: i64,
    ) -> Result<(GameFlowMutationResult, Vec<memberships::GameMembership>), AppError> {
        // Join has no expected_version in the request, so we lock it to "current version at start of txn"
        let game_before = games_repo::require_game(txn, game_id).await?;
        let expected_version = game_before.version;

        let result = self
            .run_mutation(txn, game_id, expected_version, |svc, txn| {
                Box::pin(async move {
                    svc.join_game_mutation(txn, game_id, user_id, expected_version)
                        .await
                })
            })
            .await?;

        // After commit (simulated in txn), we need memberships for the response.
        // Since we are still in the txn, we fetch them here to return to the handler.
        let memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        Ok((result, memberships))
    }

    /// Wrapper: join_as_spectator (no drain)
    pub async fn join_as_spectator(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        user_id: i64,
    ) -> Result<(GameFlowMutationResult, Vec<memberships::GameMembership>), AppError> {
        // Fetch current version for optimistic locking
        let game_before = games_repo::require_game(txn, game_id).await?;
        let expected_version = game_before.version;

        let result = self
            .run_mutation(txn, game_id, expected_version, |svc, txn| {
                Box::pin(async move {
                    svc.join_as_spectator_mutation(txn, game_id, user_id, expected_version)
                        .await
                })
            })
            .await?;

        let memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        Ok((result, memberships))
    }

    /// Wrapper: rejoin_game (no drain)
    pub async fn rejoin_game(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        user_id: i64,
        expected_version: i32,
    ) -> Result<(GameFlowMutationResult, Vec<memberships::GameMembership>), AppError> {
        let result = self
            .run_mutation(txn, game_id, expected_version, |svc, txn| {
                Box::pin(async move {
                    svc.rejoin_game_mutation(txn, game_id, user_id, expected_version)
                        .await
                })
            })
            .await?;

        let memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        Ok((result, memberships))
    }

    /// Wrapper: remove_ai_seat (no drain)
    pub async fn remove_ai_seat(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        user_id: i64, // Requesting user (must be host)
        seat: Option<u8>,
        expected_version: i32,
    ) -> Result<(GameFlowMutationResult, Vec<memberships::GameMembership>), AppError> {
        let result = self
            .run_mutation(txn, game_id, expected_version, |svc, txn| {
                Box::pin(async move {
                    svc.remove_ai_seat_mutation(txn, game_id, user_id, seat, expected_version)
                        .await
                })
            })
            .await?;

        let memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        Ok((result, memberships))
    }

    /// Wrapper: leave_game
    pub async fn leave_game(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        user_id: i64,
        expected_version: i32,
    ) -> Result<
        (
            GameFlowMutationResult,
            Vec<memberships::GameMembership>,
            bool,
        ),
        AppError,
    > {
        // We need to know if it was active to return to the handler (for orchestration)
        let game_before = games_repo::require_game(txn, game_id).await?;
        let was_active = game_before.state != DbGameState::Lobby;

        let result = self
            .run_mutation(txn, game_id, expected_version, |svc, txn| {
                Box::pin(async move {
                    svc.leave_game_mutation(txn, game_id, user_id, expected_version, was_active)
                        .await
                })
            })
            .await?;

        let memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        Ok((result, memberships, was_active))
    }

    /// Wrapper: add_ai_seat (no drain)
    pub async fn add_ai_seat(
        &self,
        txn: &DatabaseTransaction,
        params: ManageAiSeatParams,
    ) -> Result<(GameFlowMutationResult, Vec<memberships::GameMembership>), AppError> {
        let game_id = params.game_id;
        let expected_version = params.expected_version;

        let result = self
            .run_mutation(txn, game_id, expected_version, |svc, txn| {
                Box::pin(async move { svc.add_ai_seat_mutation(txn, params).await })
            })
            .await?;

        let memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        Ok((result, memberships))
    }

    /// Wrapper: update_ai_seat (no drain)
    pub async fn update_ai_seat(
        &self,
        txn: &DatabaseTransaction,
        params: ManageAiSeatParams,
    ) -> Result<(GameFlowMutationResult, Vec<memberships::GameMembership>), AppError> {
        let game_id = params.game_id;
        let expected_version = params.expected_version;

        let result = self
            .run_mutation(txn, game_id, expected_version, |svc, txn| {
                Box::pin(async move { svc.update_ai_seat_mutation(txn, params).await })
            })
            .await?;

        let memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        Ok((result, memberships))
    }

    /// Mutation: join_game
    pub(super) async fn join_game_mutation(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        user_id: i64,
        expected_version: i32,
    ) -> Result<Vec<crate::domain::game_transition::GameTransition>, AppError> {
        // Fetch game and verify it exists
        let game = games_repo::require_game(txn, game_id).await?;

        // Verify game is in LOBBY state
        if game.state != DbGameState::Lobby {
            return Err(AppError::bad_request(
                crate::errors::ErrorCode::PhaseMismatch,
                format!(
                    "Game is not in LOBBY state (current state: {:?})",
                    game.state
                ),
            ));
        }

        // Check if user is already a member
        let existing_membership = memberships::find_membership(txn, game_id, user_id)
            .await
            .map_err(AppError::from)?;
        if existing_membership.is_some() {
            return Err(AppError::Conflict {
                code: crate::errors::ErrorCode::Conflict,
                detail: format!("User {user_id} is already a member of game {game_id}"),
                extensions: None,
            });
        }

        // Get all current memberships to check capacity and find next seat
        let all_memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        let player_memberships: Vec<memberships::GameMembership> = all_memberships
            .into_iter()
            .filter(|m| m.role == GameRole::Player)
            .collect();

        let ai_players: Vec<memberships::GameMembership> = player_memberships
            .iter()
            .filter(|m| m.player_kind == crate::entities::game_players::PlayerKind::Ai)
            .cloned()
            .collect();

        let total_players = player_memberships.len();

        if total_players >= 4 {
            if ai_players.is_empty() {
                // Game is full with human players only
                return Err(AppError::bad_request(
                    crate::errors::ErrorCode::ValidationError,
                    "Game is full (maximum 4 human players)".to_string(),
                ));
            }

            // Replace the AI with the lowest seat number (turn_order)
            let ai_to_replace = ai_players
                .iter()
                .filter_map(|m| m.turn_order.map(|t| (m.clone(), t)))
                .min_by_key(|(_, t)| *t)
                .ok_or_else(|| {
                    AppError::internal(
                        crate::errors::ErrorCode::InternalError,
                        "Failed to select AI seat for replacement".to_string(),
                        std::io::Error::other("no AI seat found despite non-empty list"),
                    )
                })?;

            let (ai_membership, turn_order) = ai_to_replace;

            // Remove any overrides and the AI membership itself
            ai_overrides::delete_by_game_player_id(txn, ai_membership.id)
                .await
                .map_err(AppError::from)?;
            memberships::delete_membership(txn, ai_membership.id)
                .await
                .map_err(AppError::from)?;

            // Create human membership in the freed seat
            memberships::create_membership(
                txn,
                game_id,
                user_id,
                Some(turn_order),
                false,
                GameRole::Player,
            )
            .await
            .map_err(AppError::from)?;
        } else {
            // Seats available: find next free seat and join there
            let next_turn_order = GameService
                .find_next_available_seat(&player_memberships)
                .ok_or_else(|| {
                    AppError::internal(
                        crate::errors::ErrorCode::InternalError,
                        "No available turn order found".to_string(),
                        std::io::Error::other("turn order calculation failed"),
                    )
                })?;

            memberships::create_membership(
                txn,
                game_id,
                user_id,
                Some(next_turn_order),
                false,
                GameRole::Player,
            )
            .await
            .map_err(AppError::from)?;
        }

        // Optimistic locking / version bump (must happen inside mutation)
        // This enforces: if someone else mutated the game since expected_version, this fails.
        let _final_game = games_repo::touch_game(txn, game_id, expected_version).await?;

        Ok(vec![
            crate::domain::game_transition::GameTransition::PlayerJoined { user_id },
        ])
    }

    /// Mutation: join_as_spectator
    pub(super) async fn join_as_spectator_mutation(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        user_id: i64,
        expected_version: i32,
    ) -> Result<Vec<crate::domain::game_transition::GameTransition>, AppError> {
        // Fetch game and verify it exists
        let game = games_repo::require_game(txn, game_id).await?;

        // Only public games can be spectated
        if game.visibility != crate::entities::games::GameVisibility::Public {
            return Err(AppError::forbidden_with_code(
                crate::errors::ErrorCode::ValidationError,
                "Only public games can be spectated".to_string(),
            ));
        }

        // Check if user is already a member
        let existing_membership = memberships::find_membership(txn, game_id, user_id)
            .await
            .map_err(AppError::from)?;
        if existing_membership.is_some() {
            return Err(AppError::Conflict {
                code: crate::errors::ErrorCode::Conflict,
                detail: format!("User {user_id} is already a member of game {game_id}"),
                extensions: None,
            });
        }

        // Create spectator membership (no turn_order)
        memberships::create_membership(
            txn,
            game_id,
            user_id,
            None,  // turn_order: None for spectators
            false, // is_ready: false (spectators don't need to be ready)
            GameRole::Spectator,
        )
        .await
        .map_err(AppError::from)?;

        // Optimistic locking / version bump
        let _final_game = games_repo::touch_game(txn, game_id, expected_version).await?;

        Ok(vec![
            crate::domain::game_transition::GameTransition::PlayerJoined { user_id },
        ])
    }

    /// Mutation: rejoin_game
    pub(super) async fn rejoin_game_mutation(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        user_id: i64,
        expected_version: i32,
    ) -> Result<Vec<crate::domain::game_transition::GameTransition>, AppError> {
        // Find AI membership with this user as original_user_id
        let all_memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;
        let ai_membership = all_memberships
            .iter()
            .find(|m| {
                m.player_kind == crate::entities::game_players::PlayerKind::Ai
                    && m.original_user_id == Some(user_id)
            })
            .ok_or_else(|| {
                AppError::bad_request(
                    crate::errors::ErrorCode::ValidationError,
                    format!("No AI seat found for user {user_id} to rejoin"),
                )
            })?;

        // Convert AI back to human immediately
        let mut updated_membership = ai_membership.clone();
        updated_membership.player_kind = crate::entities::game_players::PlayerKind::Human;
        updated_membership.user_id = Some(user_id);
        updated_membership.original_user_id = None; // Clear after rejoining
        updated_membership.ai_profile_id = None;

        // Delete AI override (display name)
        ai_overrides::delete_by_game_player_id(txn, updated_membership.id)
            .await
            .map_err(AppError::from)?;

        let _updated = memberships::update_membership(txn, updated_membership)
            .await
            .map_err(AppError::from)?;

        // Optimistic locking / version bump
        let _final_game = games_repo::touch_game(txn, game_id, expected_version).await?;

        Ok(vec![
            crate::domain::game_transition::GameTransition::PlayerRejoined { user_id },
        ])
    }

    /// Mutation: remove_ai_seat
    pub(super) async fn remove_ai_seat_mutation(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        user_id: i64,
        seat: Option<u8>,
        expected_version: i32,
    ) -> Result<Vec<crate::domain::game_transition::GameTransition>, AppError> {
        let game = games_repo::require_game(txn, game_id).await?;

        // Host validation
        if !GameService.is_host(&game, Some(user_id)) {
            return Err(AppError::forbidden_with_code(
                crate::errors::ErrorCode::Forbidden,
                "Only the host can manage AI seats",
            ));
        }

        // Phase validation
        if game.state != DbGameState::Lobby {
            return Err(AppError::bad_request(
                crate::errors::ErrorCode::PhaseMismatch,
                "AI seats can only be managed before the game starts",
            ));
        }

        let existing_memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        let candidate = if let Some(target_seat) = seat {
            if target_seat > 3 {
                return Err(AppError::bad_request(
                    crate::errors::ErrorCode::InvalidSeat,
                    format!("Seat {target_seat} is out of range (0-3)"),
                ));
            }

            let m = existing_memberships
                .iter()
                .find(|m| m.turn_order == Some(target_seat))
                .cloned()
                .ok_or_else(|| {
                    AppError::bad_request(
                        crate::errors::ErrorCode::ValidationError,
                        format!("No player assigned to seat {target_seat}"),
                    )
                })?;

            if m.player_kind != crate::entities::game_players::PlayerKind::Ai {
                return Err(AppError::bad_request(
                    crate::errors::ErrorCode::ValidationError,
                    "Specified seat is not occupied by an AI player",
                ));
            }
            m
        } else {
            let mut ai_memberships: Vec<_> = existing_memberships
                .iter()
                .filter(|m| m.player_kind == crate::entities::game_players::PlayerKind::Ai)
                .cloned()
                .collect();

            if ai_memberships.is_empty() {
                return Err(AppError::bad_request(
                    crate::errors::ErrorCode::ValidationError,
                    "There are no AI seats to remove",
                ));
            }

            ai_memberships.sort_by_key(|m| m.turn_order);
            ai_memberships.pop().ok_or_else(|| {
                AppError::internal(
                    crate::errors::ErrorCode::InternalError,
                    "Failed to select AI membership for removal",
                    std::io::Error::other("No AI membership found after check"),
                )
            })?
        };

        // Remove AI + overrides
        ai_overrides::delete_by_game_player_id(txn, candidate.id)
            .await
            .map_err(AppError::from)?;
        memberships::delete_membership(txn, candidate.id)
            .await
            .map_err(AppError::from)?;

        // Optimistic locking / version bump
        let _final_game = games_repo::touch_game(txn, game_id, expected_version).await?;

        Ok(vec![])
    }

    /// Mutation: add_ai_seat
    pub(super) async fn add_ai_seat_mutation(
        &self,
        txn: &DatabaseTransaction,
        params: ManageAiSeatParams,
    ) -> Result<Vec<crate::domain::game_transition::GameTransition>, AppError> {
        let game_id = params.game_id;
        let user_id = params.user_id;
        let expected_version = params.expected_version;

        let game = games_repo::require_game(txn, game_id).await?;

        // Host validation
        if !GameService.is_host(&game, Some(user_id)) {
            return Err(AppError::forbidden_with_code(
                crate::errors::ErrorCode::Forbidden,
                "Only the host can manage AI seats",
            ));
        }

        // Phase validation
        if game.state != DbGameState::Lobby {
            return Err(AppError::bad_request(
                crate::errors::ErrorCode::PhaseMismatch,
                "AI seats can only be managed before the game starts",
            ));
        }

        let existing_memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        // Count only players (exclude spectators)
        let player_count = existing_memberships
            .iter()
            .filter(|m| m.role == memberships::GameRole::Player)
            .count();

        if player_count >= 4 {
            return Err(AppError::conflict(
                crate::errors::ErrorCode::SeatTaken,
                "All seats are already filled",
            ));
        }

        let seat_to_fill = if let Some(target_seat) = params.seat {
            if target_seat > 3 {
                return Err(AppError::bad_request(
                    crate::errors::ErrorCode::InvalidSeat,
                    format!("Seat {target_seat} is out of range (0-3)"),
                ));
            }
            if existing_memberships
                .iter()
                .any(|m| m.turn_order == Some(target_seat))
            {
                return Err(AppError::conflict(
                    crate::errors::ErrorCode::SeatTaken,
                    format!("Seat {target_seat} is already taken"),
                ));
            }
            target_seat
        } else {
            // Find first available seat (numerically lowest)
            let mut occupied_seats = std::collections::HashSet::new();
            for m in &existing_memberships {
                if let Some(s) = m.turn_order {
                    occupied_seats.insert(s);
                }
            }
            // clippy::let_and_return
            (0..4)
                .find(|s| !occupied_seats.contains(s))
                .ok_or_else(|| {
                    AppError::conflict(crate::errors::ErrorCode::SeatTaken, "No seats available")
                })?
        };

        // Resolve profile and configuration
        let (profile, seed) = self.resolve_ai_details(txn, &params).await?;

        // Collect existing names for uniqueness
        let existing_names = self
            .collect_existing_display_names(txn, game_id, None)
            .await?;
        let unique_name = self.generate_unique_ai_name(&existing_names, seat_to_fill);

        // Create membership
        let created_membership = memberships::create_ai_membership(
            txn,
            game_id,
            profile.id,
            seat_to_fill,
            true, // is_ready
            memberships::GameRole::Player,
        )
        .await
        .map_err(AppError::from)?;

        // Add overrides (name + seed)
        let config = seed.map(|s| {
            let mut cfg = serde_json::Map::new();
            cfg.insert("seed".to_string(), serde_json::Value::Number(s.into()));
            serde_json::Value::Object(cfg)
        });

        ai_overrides::create_override(txn, created_membership.id, Some(unique_name), None, config)
            .await
            .map_err(AppError::from)?;

        // Automatic game start check
        self.check_and_start_game_if_ready(txn, game_id).await?;

        // Optimistic locking / version bump
        let _final_game = games_repo::touch_game(txn, game_id, expected_version).await?;

        Ok(vec![])
    }

    /// Mutation: update_ai_seat
    pub(super) async fn update_ai_seat_mutation(
        &self,
        txn: &DatabaseTransaction,
        params: ManageAiSeatParams,
    ) -> Result<Vec<crate::domain::game_transition::GameTransition>, AppError> {
        let game_id = params.game_id;
        let user_id = params.user_id;
        let seat = params.seat.ok_or_else(|| {
            AppError::bad_request(
                crate::errors::ErrorCode::ValidationError,
                "seat is required for updating AI",
            )
        })?;
        let expected_version = params.expected_version;

        let game = games_repo::require_game(txn, game_id).await?;

        // Host validation
        if !GameService.is_host(&game, Some(user_id)) {
            return Err(AppError::forbidden_with_code(
                crate::errors::ErrorCode::Forbidden,
                "Only the host can manage AI seats",
            ));
        }

        // Phase validation
        if game.state != DbGameState::Lobby {
            return Err(AppError::bad_request(
                crate::errors::ErrorCode::PhaseMismatch,
                "AI seats can only be managed before the game starts",
            ));
        }

        let existing_memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        let membership = existing_memberships
            .into_iter()
            .find(|m| m.turn_order == Some(seat))
            .ok_or_else(|| {
                AppError::bad_request(
                    crate::errors::ErrorCode::ValidationError,
                    format!("No player assigned to seat {seat}"),
                )
            })?;

        if membership.player_kind != crate::entities::game_players::PlayerKind::Ai {
            return Err(AppError::bad_request(
                crate::errors::ErrorCode::ValidationError,
                "Cannot update AI profile for a human player",
            ));
        }

        let (new_profile, seed) = self.resolve_ai_details(txn, &params).await?;

        let mut updated_membership = membership.clone();
        updated_membership.ai_profile_id = Some(new_profile.id);
        memberships::update_membership(txn, updated_membership)
            .await
            .map_err(AppError::from)?;

        if let Some(seed_value) = seed {
            let mut cfg = serde_json::Map::new();
            cfg.insert(
                "seed".to_string(),
                serde_json::Value::Number(seed_value.into()),
            );
            let cfg_value = serde_json::Value::Object(cfg);

            if let Some(mut existing_override) =
                ai_overrides::find_by_game_player_id(txn, membership.id)
                    .await
                    .map_err(AppError::from)?
            {
                existing_override.config = Some(cfg_value);
                ai_overrides::update_override(txn, existing_override)
                    .await
                    .map_err(AppError::from)?;
            } else {
                ai_overrides::create_override(txn, membership.id, None, None, Some(cfg_value))
                    .await
                    .map_err(AppError::from)?;
            }
        }

        // Optimistic locking / version bump
        let _final_game = games_repo::touch_game(txn, game_id, expected_version).await?;

        Ok(vec![])
    }

    /// Mutation: leave_game
    pub(super) async fn leave_game_mutation(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        user_id: i64,
        expected_version: i32,
        was_active: bool,
    ) -> Result<Vec<crate::domain::game_transition::GameTransition>, AppError> {
        // Find the user's membership
        let membership = memberships::find_membership(txn, game_id, user_id)
            .await?
            .ok_or_else(|| {
                AppError::bad_request(
                    crate::errors::ErrorCode::ValidationError,
                    format!("User {user_id} is not a member of game {game_id}"),
                )
            })?;

        // Spectators can always just be removed (no seat to replace)
        if membership.role == GameRole::Spectator {
            memberships::delete_membership(txn, membership.id)
                .await
                .map_err(AppError::from)?;
        } else if !was_active {
            // If game is in Lobby, delete the membership
            memberships::delete_membership(txn, membership.id)
                .await
                .map_err(AppError::from)?;
        } else {
            // Game is active: convert human to AI
            self.convert_human_to_ai_mutation(txn, membership).await?;
        }

        // Optimistic locking / version bump
        let _final_game = games_repo::touch_game(txn, game_id, expected_version).await?;

        Ok(vec![
            crate::domain::game_transition::GameTransition::PlayerLeft { user_id },
        ])
    }

    /// Helper: Convert a human player to an AI player (no touch, just the membership flip)
    async fn convert_human_to_ai_mutation(
        &self,
        txn: &DatabaseTransaction,
        membership: memberships::GameMembership,
    ) -> Result<memberships::GameMembership, AppError> {
        // Find the default AI profile
        let ai_profile = GameService.find_default_ai_profile(txn).await?;

        // Collect existing display names to ensure uniqueness
        let existing_display_names = self
            .collect_existing_display_names(txn, membership.game_id, Some(membership.id))
            .await?;

        let seat = membership.turn_order.ok_or_else(|| {
            AppError::internal(
                crate::errors::ErrorCode::InternalError,
                "AI player must have turn_order".to_string(),
                std::io::Error::other("AI player without turn_order"),
            )
        })?;

        let unique_name = self.generate_unique_ai_name(&existing_display_names, seat);

        // Update membership to AI
        let mut updated_membership = membership.clone();
        updated_membership.player_kind = crate::entities::game_players::PlayerKind::Ai;
        updated_membership.original_user_id = membership.user_id; // Store original user
        updated_membership.user_id = None;
        updated_membership.ai_profile_id = Some(ai_profile.id);

        let updated = memberships::update_membership(txn, updated_membership)
            .await
            .map_err(AppError::from)?;

        // Create AI override for the display name
        ai_overrides::create_override(txn, updated.id, Some(unique_name), None, None)
            .await
            .map_err(AppError::from)?;

        Ok(updated)
    }

    /// Helper: Collect all existing display names for a game.
    async fn collect_existing_display_names(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        exclude_membership_id: Option<i64>,
    ) -> Result<std::collections::HashSet<String>, AppError> {
        let all_memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;
        let mut existing_display_names = std::collections::HashSet::new();

        for m in &all_memberships {
            if Some(m.id) == exclude_membership_id {
                continue;
            }
            if let Some(seat) = m.turn_order {
                let display_name = resolve_display_name_for_membership(
                    txn, m, seat, false, // No final fallback needed
                )
                .await
                .map_err(AppError::from)?;
                existing_display_names.insert(display_name);
            }
        }
        Ok(existing_display_names)
    }

    /// Helper: Generate a unique AI display name.
    fn generate_unique_ai_name(
        &self,
        existing: &std::collections::HashSet<String>,
        seat: u8,
    ) -> String {
        use rand::random;
        let name_seed = random::<u64>() as i64;
        let base_name = friendly_ai_name(name_seed, seat as usize);

        if !existing.contains(&base_name) {
            return base_name;
        }

        let mut counter = 2;
        loop {
            let candidate = format!("{base_name} {counter}");
            if !existing.contains(&candidate) {
                return candidate;
            }
            counter += 1;
        }
    }

    /// Helper: Resolve AI profile and configuration based on registry name/version/seed.
    /// Handles default values if not provided.
    async fn resolve_ai_details(
        &self,
        txn: &DatabaseTransaction,
        params: &ManageAiSeatParams,
    ) -> Result<(ai_profiles::AiProfile, Option<u64>), AppError> {
        use rand::random;

        use crate::ai::{registry, RandomPlayer};

        let name = params
            .registry_name
            .as_deref()
            .unwrap_or(registry::DEFAULT_AI_NAME);

        let factory = registry::by_name(name).ok_or_else(|| {
            AppError::bad_request(
                crate::errors::ErrorCode::ValidationError,
                format!("Unknown AI registry entry '{name}'"),
            )
        })?;

        let registry_version = match params.registry_version.as_deref() {
            Some(v) => {
                if v != factory.version {
                    return Err(AppError::bad_request(
                        crate::errors::ErrorCode::ValidationError,
                        format!(
                            "Registry version '{}' does not match server version '{}' for '{}'",
                            v, factory.version, factory.name
                        ),
                    ));
                }
                v
            }
            None => factory.version,
        };

        let mut seed = params.config_seed;
        if seed.is_none() && factory.name == RandomPlayer::NAME {
            seed = Some(random::<u64>());
        }

        let profile = ai_profiles::find_by_registry_variant(txn, name, registry_version, "default")
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| {
                AppError::bad_request(
                    crate::errors::ErrorCode::ValidationError,
                    format!("AI profile for {name} v{registry_version} not found"),
                )
            })?;

        Ok((profile, seed))
    }
}
