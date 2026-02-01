//! Game state loading and construction services.

use sea_orm::{
    ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

use crate::adapters::games_sea::GameCreate;
use crate::domain::cards_parsing::from_stored_format;
use crate::domain::state::{
    expected_actor, nth_from, round_start_seat, GameState, Phase, PreviousRound, RoundState,
};
use crate::domain::Card;
use crate::entities::game_players;
use crate::entities::games::{self, GameState as DbGameState, GameVisibility};
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::games::Game;
use crate::repos::memberships::GameRole;
use crate::repos::{
    ai_profiles, bids, games as games_repo, hands, memberships, plays, rounds, scores, tricks,
};
use crate::services::game_flow::GameFlowService;

/// Game domain service (stateless).
#[derive(Default)]
pub struct GameService;

impl GameService {
    /// Load GameState from database (requires transaction for consistent snapshot).
    ///
    /// Reconstructs in-memory GameState by loading all persisted data for the current round.
    pub async fn load_game_state(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<GameState, AppError> {
        // 1. Load game record
        let game = games_repo::require_game(txn, game_id).await?;

        // 2. Check if game has started (has a current round)
        let current_round_no = match game.current_round {
            Some(round_no) => round_no,
            None => {
                // Game hasn't started yet - return empty initial state (no rounds exist)
                let phase = games_repo::db_game_state_to_phase(&game.state, game.current_trick_no);

                return Ok(GameState {
                    phase,
                    round_no: None,
                    hand_size: None,
                    hands: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
                    dealer: None,
                    turn: None,
                    leader: None,
                    trick_no: None,
                    scores_total: [0, 0, 0, 0],
                    round: RoundState::empty(),
                });
            }
        };

        let hand_size = game.hand_size().ok_or_else(|| {
            DomainError::validation(ValidationKind::InvalidHandSize, "Hand size not set")
        })?;

        // 3. Load round record
        let round = rounds::find_by_game_and_round(txn, game_id, current_round_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("ROUND_NOT_FOUND".into()),
                    "Round not found",
                )
            })?;

        // 4. Load player hands
        let all_hands = hands::find_all_by_round(txn, round.id).await?;
        let mut hands_array: [Vec<Card>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];

        for hand in all_hands {
            if hand.player_seat < 4 {
                let domain_cards = hand
                    .cards
                    .iter()
                    .map(|c| from_stored_format(&c.suit, &c.rank))
                    .collect::<Result<Vec<_>, _>>()?;
                hands_array[hand.player_seat as usize] = domain_cards;
            }
        }

        // 5. Load bids
        let all_bids = bids::find_all_by_round(txn, round.id).await?;
        let mut bids_array = [None, None, None, None];
        for bid in &all_bids {
            if bid.player_seat < 4 {
                bids_array[bid.player_seat as usize] = Some(bid.bid_value);
            }
        }

        // 6. Determine winning bidder
        let winning_bidder = if bids_array.iter().all(|b| b.is_some()) {
            bids::find_winning_bid(txn, round.id)
                .await?
                .map(|b| b.player_seat)
        } else {
            None
        };

        // 7. Load trump (already domain type, no conversion needed)
        let trump = round.trump;

        // 8. Count tricks won
        let all_tricks = tricks::find_all_by_round(txn, round.id).await?;
        let mut tricks_won = [0u8; 4];
        for trick in &all_tricks {
            let winner = trick.winner_seat;
            if !(0..=3).contains(&winner) {
                continue;
            }

            // Skip placeholder winners for the in-progress trick (winner is u8::MAX until trick is resolved)
            if game.state == DbGameState::TrickPlay
                && trick.trick_no == game.current_trick_no
                && winner == u8::MAX
            {
                continue;
            }

            tricks_won[winner as usize] += 1;
        }

        // 8a. Capture previous round summary (for start-of-round display) and its final trick
        let mut previous_round_summary: Option<PreviousRound> = None;
        let mut previous_round_last_trick: Option<Vec<(u8, Card)>> = None;

        if current_round_no > 1
            && matches!(
                game.state,
                DbGameState::Bidding | DbGameState::TrumpSelection
            )
        {
            let prev_round_no = current_round_no - 1;
            if let Some(prev_round) =
                rounds::find_by_game_and_round(txn, game_id, prev_round_no).await?
            {
                let prev_round_tricks = tricks::find_all_by_round(txn, prev_round.id).await?;
                let mut prev_tricks_won = [0u8; 4];
                let mut prev_last_trick_meta: Option<(i16, i64)> = None;

                for trick in &prev_round_tricks {
                    let winner = trick.winner_seat;
                    if (0..=3).contains(&winner) {
                        prev_tricks_won[winner as usize] += 1;
                        let should_replace = prev_last_trick_meta
                            .map(|(existing_no, _)| i16::from(trick.trick_no) > existing_no)
                            .unwrap_or(true);
                        if should_replace {
                            prev_last_trick_meta = Some((i16::from(trick.trick_no), trick.id));
                        }
                    }
                }

                let prev_bids_models = bids::find_all_by_round(txn, prev_round.id).await?;
                let mut prev_bids = [None, None, None, None];
                for bid in prev_bids_models {
                    if bid.player_seat < 4 {
                        prev_bids[bid.player_seat as usize] = Some(bid.bid_value);
                    }
                }

                // Compute hand_size from round_no
                let prev_hand_size =
                    crate::domain::rules::hand_size_for_round(prev_round_no).unwrap_or(0);

                previous_round_summary = Some(PreviousRound {
                    round_no: prev_round_no,
                    hand_size: prev_hand_size,
                    tricks_won: prev_tricks_won,
                    bids: prev_bids,
                });

                if let Some((_, trick_id)) = prev_last_trick_meta {
                    let prev_plays = plays::find_all_by_trick(txn, trick_id).await?;
                    previous_round_last_trick = (prev_plays.len() == 4)
                        .then(|| {
                            prev_plays
                                .iter()
                                .map(|p| {
                                    let card = from_stored_format(&p.card.suit, &p.card.rank)?;
                                    Ok((p.player_seat, card))
                                })
                                .collect::<Result<Vec<_>, DomainError>>()
                        })
                        .transpose()?;
                }
            }
        }

        // Remove all played cards from players' hands to reflect current state
        for trick in &all_tricks {
            let play_records = plays::find_all_by_trick(txn, trick.id).await?;
            for play in play_records {
                let seat = play.player_seat;
                if !(0..=3).contains(&seat) {
                    continue;
                }
                let card = from_stored_format(&play.card.suit, &play.card.rank)?;
                if let Some(hand) = hands_array.get_mut(seat as usize) {
                    if let Some(pos) = hand.iter().position(|c| *c == card) {
                        hand.remove(pos);
                    }
                }
            }
        }

        // 9. Load current trick plays (if in TrickPlay)
        let current_trick_no = game.current_trick_no;
        let (trick_plays, trick_lead) = if let DbGameState::TrickPlay = game.state {
            if let Some(current_trick) =
                tricks::find_by_round_and_trick(txn, round.id, current_trick_no).await?
            {
                let all_plays = plays::find_all_by_trick(txn, current_trick.id).await?;

                let plays = all_plays
                    .iter()
                    .map(|p| {
                        let card = from_stored_format(&p.card.suit, &p.card.rank)?;
                        Ok((p.player_seat, card))
                    })
                    .collect::<Result<Vec<_>, DomainError>>()?;

                // lead_suit is already domain type, no conversion needed
                let lead = current_trick.lead_suit;

                (plays, Some(lead))
            } else {
                (Vec::new(), None)
            }
        } else {
            (Vec::new(), None)
        };

        // 9a. Load last completed trick
        // - If in TrickPlay: load last trick from current round
        // - If in Bidding/TrumpSelect: reuse final trick from previous round summary
        let last_trick = if matches!(game.state, DbGameState::TrickPlay) {
            // Current round, previous trick
            let prev_trick_id = all_tricks
                .iter()
                .filter(|t| t.trick_no < current_trick_no && (0..=3).contains(&t.winner_seat))
                .max_by_key(|t| t.trick_no)
                .map(|prev_trick| prev_trick.id);

            if let Some(trick_id) = prev_trick_id {
                let prev_plays = plays::find_all_by_trick(txn, trick_id).await?;
                (prev_plays.len() == 4)
                    .then(|| {
                        prev_plays
                            .iter()
                            .map(|p| {
                                let card = from_stored_format(&p.card.suit, &p.card.rank)?;
                                Ok((p.player_seat, card))
                            })
                            .collect::<Result<Vec<_>, DomainError>>()
                    })
                    .transpose()?
            } else {
                None
            }
        } else if matches!(
            game.state,
            DbGameState::Bidding | DbGameState::TrumpSelection
        ) {
            previous_round_last_trick
        } else {
            None
        };

        // 10. Load cumulative scores
        let scores_total = scores::get_current_totals(txn, game_id).await?;

        // 11. Convert DB phase to domain Phase
        let phase = games_repo::db_game_state_to_phase(&game.state, current_trick_no);

        // 12. Determine dealer, leader, turn, and optional round fields (honest; no sentinels)
        let dealer: Option<u8> = game.dealer_pos();

        let last_completed_trick_winner: Option<u8> = all_tricks
            .iter()
            .rev()
            .find(|t| (0..=3).contains(&t.winner_seat))
            .map(|t| t.winner_seat);

        let (round_no_opt, hand_size_opt, trick_no_opt, leader_opt, turn_opt) = match phase {
            Phase::Init => (None, None, None, None, None),

            Phase::Bidding => {
                let round_no = Some(current_round_no);
                let hand_size = Some(hand_size);

                let dealer_seat = dealer.ok_or_else(|| {
                    DomainError::validation_other(
                        "Invariant violated: dealer must be set during Bidding",
                    )
                })?;
                let round_start = round_start_seat(dealer_seat);

                let bid_count = all_bids.len() as u8;
                let to_act = nth_from(round_start, bid_count);

                (round_no, hand_size, None, None, Some(to_act))
            }

            Phase::TrumpSelect => {
                let round_no = Some(current_round_no);
                let hand_size = Some(hand_size);

                // TrumpSelect is actionable only for the winning bidder; if missing, no turn.
                let to_act = winning_bidder;

                (round_no, hand_size, None, None, to_act)
            }

            Phase::Trick { trick_no } => {
                let round_no = Some(current_round_no);
                let hand_size = Some(hand_size);

                let leader = if let Some((seat, _)) = trick_plays.first() {
                    Some(*seat)
                } else if let Some(winner) = last_completed_trick_winner {
                    Some(winner)
                } else {
                    // First trick, no plays yet: leader is left of dealer
                    let dealer_seat = dealer.ok_or_else(|| {
                        DomainError::validation_other(
                            "Invariant violated: dealer must be set during Trick",
                        )
                    })?;
                    Some(round_start_seat(dealer_seat))
                };

                let to_act = leader.map(|l| {
                    let plays_count = trick_plays.len() as u8;
                    expected_actor(l, plays_count)
                });

                (round_no, hand_size, Some(trick_no), leader, to_act)
            }

            Phase::Scoring => (Some(current_round_no), Some(hand_size), None, None, None),
            Phase::Complete => (Some(current_round_no), Some(hand_size), None, None, None),
            Phase::GameOver => (Some(current_round_no), None, None, None, None),
        };

        Ok(GameState {
            phase,
            round_no: round_no_opt,
            hand_size: hand_size_opt,
            hands: hands_array,
            dealer,
            turn: turn_opt,
            leader: leader_opt,
            trick_no: trick_no_opt,
            scores_total,
            round: RoundState {
                trick_plays,
                trick_lead,
                tricks_won,
                trump,
                bids: bids_array,
                winning_bidder,
                last_trick,
                previous_round: previous_round_summary,
            },
        })
    }

    /// Create a game with the creator as the first member.
    ///
    /// This method handles:
    /// - Creating the game
    /// - Adding the creator as a member (turn_order: 0)
    /// - Returning the game entity and all memberships
    ///
    /// All operations are performed within the provided transaction.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `user_id` - ID of the user creating the game
    /// * `name` - Optional game name
    ///
    /// # Returns
    /// Tuple of (game entity model, all memberships)
    pub async fn create_game_with_creator(
        &self,
        txn: &DatabaseTransaction,
        user_id: i64,
        name: Option<String>,
    ) -> Result<(Game, Vec<memberships::GameMembership>), AppError> {
        // Create a new public game without join codes and add the creator as the first member.
        let dto = GameCreate::new()
            .with_visibility(GameVisibility::Public)
            .by(user_id);

        let dto = if let Some(ref n) = name {
            dto.with_name(n.clone())
        } else {
            dto
        };

        let game = games_repo::create_game(txn, dto)
            .await
            .map_err(AppError::from)?;

        // Add creator as first member
        memberships::create_membership(
            txn,
            game.id,
            user_id,
            Some(0), // turn_order: 0 for creator
            false,   // is_ready: false
            GameRole::Player,
        )
        .await
        .map_err(AppError::from)?;

        // Fetch game and all memberships
        let all_memberships = memberships::find_all_by_game(txn, game.id)
            .await
            .map_err(AppError::from)?;

        Ok((game, all_memberships))
    }

    /// List all public games in the lobby that still have open seats.
    ///
    /// Returns games along with their memberships so the caller can compute
    /// player counts and viewer-specific flags.
    pub async fn list_joinable_games(
        &self,
        txn: &DatabaseTransaction,
    ) -> Result<Vec<(Game, Vec<memberships::GameMembership>)>, AppError> {
        let lobby_games = games::Entity::find()
            .filter(games::Column::State.eq(DbGameState::Lobby))
            .order_by_desc(games::Column::UpdatedAt)
            .all(txn)
            .await
            .map_err(AppError::from)?;

        let mut results = Vec::new();
        for game_model in lobby_games {
            let game = Game::from(game_model);
            let memberships = memberships::find_all_by_game(txn, game.id)
                .await
                .map_err(AppError::from)?;

            let player_count = memberships
                .iter()
                .filter(|m| m.role == GameRole::Player)
                .count();

            // Limit joinable list to public games with open seats
            if player_count < 4 && game.visibility == GameVisibility::Public {
                results.push((game, memberships));
            }
        }

        Ok(results)
    }

    /// List all public games in the lobby regardless of seat availability.
    pub async fn list_public_lobby_games(
        &self,
        txn: &DatabaseTransaction,
    ) -> Result<Vec<(Game, Vec<memberships::GameMembership>)>, AppError> {
        let lobby_games = games::Entity::find()
            .filter(games::Column::State.eq(DbGameState::Lobby))
            .order_by_desc(games::Column::UpdatedAt)
            .all(txn)
            .await
            .map_err(AppError::from)?;

        let mut results = Vec::new();
        for game_model in lobby_games {
            let game = Game::from(game_model);
            if game.visibility != GameVisibility::Public {
                continue;
            }

            let memberships = memberships::find_all_by_game(txn, game.id)
                .await
                .map_err(AppError::from)?;
            results.push((game, memberships));
        }

        Ok(results)
    }

    /// List all games that are actively in progress (non-lobby, non-finished states).
    ///
    /// Includes memberships so the caller can determine viewer participation.
    pub async fn list_active_games(
        &self,
        txn: &DatabaseTransaction,
    ) -> Result<Vec<(Game, Vec<memberships::GameMembership>)>, AppError> {
        let active_states = [
            DbGameState::Dealing,
            DbGameState::Bidding,
            DbGameState::TrumpSelection,
            DbGameState::TrickPlay,
            DbGameState::Scoring,
            DbGameState::BetweenRounds,
        ];

        let active_games = games::Entity::find()
            .filter(games::Column::State.is_in(active_states))
            .order_by_desc(games::Column::UpdatedAt)
            .all(txn)
            .await
            .map_err(AppError::from)?;

        let mut results = Vec::with_capacity(active_games.len());
        for game_model in active_games {
            let game = Game::from(game_model);
            let memberships = memberships::find_all_by_game(txn, game.id)
                .await
                .map_err(AppError::from)?;
            results.push((game, memberships));
        }

        Ok(results)
    }

    /// Find the game that has been waiting for the user to act the longest.
    ///
    /// Prioritizes games where:
    /// - The user is a player (not spectator)
    /// - It is currently the user's turn (Bidding, TrumpSelection, or TrickPlay)
    /// - The game has been waiting the longest (oldest updated_at)
    ///
    /// If no games are waiting for the user, falls back to the most recently active game
    /// (highest updated_at timestamp) where the user is a member.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `user_id` - ID of the user
    /// * `exclude_game_id` - Optional ID of a game to exclude
    ///
    /// # Returns
    /// Option<i64> - Game ID if found, None if user has no games
    /// Find games that have been waiting for the user to act the longest.
    ///
    /// Prioritizes games where:
    /// - The user is a player (not spectator)
    /// - It is currently the user's turn (Bidding, TrumpSelection, or TrickPlay)
    /// - The game has been waiting the longest (oldest updated_at)
    ///
    /// Returns up to 2 games to allow client-side optimistic switching.
    ///
    /// If no games are waiting for the user, falls back to the most recently active game
    /// (highest updated_at timestamp) where the user is a member.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `user_id` - ID of the user
    ///
    /// # Returns
    /// Vec<i64> - List of Game IDs (empty if none found)
    pub async fn game_waiting_longest(
        &self,
        txn: &DatabaseTransaction,
        user_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        // Find all games where user is a player (not spectator)
        let memberships = game_players::Entity::find()
            .filter(game_players::Column::PlayerKind.eq(game_players::PlayerKind::Human))
            .filter(game_players::Column::HumanUserId.eq(user_id))
            .filter(game_players::Column::Role.eq(game_players::GameRole::Player))
            .all(txn)
            .await
            .map_err(AppError::from)?;

        if memberships.is_empty() {
            return Ok(Vec::new());
        }

        // Build map of game_id -> turn_order for efficient lookup
        let mut game_id_to_turn_order: std::collections::HashMap<i64, u8> =
            std::collections::HashMap::new();
        for membership in &memberships {
            if let Some(turn_order) = membership.turn_order {
                if let Ok(turn_order_u8) = u8::try_from(turn_order) {
                    game_id_to_turn_order.insert(membership.game_id, turn_order_u8);
                }
            }
        }

        // Get all games where user is a member, filtered to actionable states only
        // (Bidding, TrumpSelection, TrickPlay - skip Lobby, BetweenRounds, etc.)
        let actionable_states = [
            DbGameState::Bidding,
            DbGameState::TrumpSelection,
            DbGameState::TrickPlay,
        ];
        let game_ids: Vec<i64> = memberships.iter().map(|m| m.game_id).collect();

        // Find which of these games contain other human players (non-spectator)
        // "Other human" means: Human player, role=Player, human_user_id != current user.
        // We only care about games the current user is already a member of (game_ids).
        let game_ids_with_other_humans: Vec<i64> = game_players::Entity::find()
            .select_only()
            .column(game_players::Column::GameId)
            .filter(game_players::Column::GameId.is_in(game_ids.clone()))
            .filter(game_players::Column::Role.eq(game_players::GameRole::Player))
            .filter(game_players::Column::PlayerKind.eq(game_players::PlayerKind::Human))
            .filter(game_players::Column::HumanUserId.ne(user_id))
            .distinct()
            .into_tuple::<i64>()
            .all(txn)
            .await
            .map_err(AppError::from)?;

        let actionable_games_with_humans = if game_ids_with_other_humans.is_empty() {
            Vec::new()
        } else {
            games::Entity::find()
                .filter(games::Column::Id.is_in(game_ids_with_other_humans.clone()))
                .filter(games::Column::State.is_in(actionable_states.clone()))
                .order_by_asc(games::Column::UpdatedAt)
                .all(txn)
                .await
                .map_err(AppError::from)?
        };

        let actionable_games_ai_only = {
            let mut q = games::Entity::find()
                .filter(games::Column::Id.is_in(game_ids.clone()))
                .filter(games::Column::State.is_in(actionable_states.clone()))
                .order_by_asc(games::Column::UpdatedAt);

            if !game_ids_with_other_humans.is_empty() {
                q = q.filter(games::Column::Id.is_not_in(game_ids_with_other_humans));
            }

            q.all(txn).await.map_err(AppError::from)?
        };

        // Check each actionable game (oldest first) to see if it's the user's turn
        // Prefer games with other humans first.
        let game_flow_service = GameFlowService;
        let mut result_ids = Vec::new();

        for game_model in actionable_games_with_humans
            .iter()
            .chain(actionable_games_ai_only.iter())
        {
            let game = games_repo::Game::from(game_model.clone());
            let user_turn_order = match game_id_to_turn_order.get(&game.id) {
                Some(&order) => order,
                None => continue,
            };

            match game_flow_service.determine_next_action(txn, &game).await {
                Ok(Some((seat, _))) if seat == user_turn_order => {
                    result_ids.push(game.id);
                    if result_ids.len() >= 2 {
                        return Ok(result_ids);
                    }
                }
                _ => continue,
            }
        }

        if !result_ids.is_empty() {
            return Ok(result_ids);
        }

        // No games waiting for the user - fall back to most recently active game
        let last_active_game = games::Entity::find()
            .filter(games::Column::Id.is_in(game_ids))
            .filter(games::Column::State.ne(DbGameState::Completed))
            .filter(games::Column::State.ne(DbGameState::Abandoned))
            .order_by_desc(games::Column::UpdatedAt)
            .one(txn)
            .await
            .map_err(AppError::from)?;

        Ok(last_active_game
            .map(|game| vec![game.id])
            .unwrap_or_default())
    }

    /// Find the next available turn_order (0-3) for a game.
    ///
    /// # Arguments
    /// * `memberships` - Current memberships for the game
    ///
    /// # Returns
    /// The next available turn_order, or None if all seats are taken
    pub fn find_next_available_seat(
        &self,
        memberships: &[memberships::GameMembership],
    ) -> Option<u8> {
        let used_turn_orders: std::collections::HashSet<u8> =
            memberships.iter().filter_map(|m| m.turn_order).collect();

        (0..4u8).find(|&order| !used_turn_orders.contains(&order))
    }

    /// Check if a user is the host of a game.
    ///
    /// A user is the host if the game's `created_by` matches the user's ID.
    /// Both `created_by` and `user_id` must be present - no legacy fallbacks.
    ///
    /// # Arguments
    /// * `game` - The game to check
    /// * `user_id` - User ID to check (must be Some)
    ///
    /// # Returns
    /// `true` if the user is the host, `false` otherwise
    pub fn is_host(&self, game: &Game, user_id: Option<i64>) -> bool {
        match (game.created_by, user_id) {
            (Some(created_by), Some(host_id)) => created_by == host_id,
            _ => false,
        }
    }

    /// Find the default AI profile.
    ///
    /// Returns the AI profile for the default AI player.
    /// This is used both for player-to-AI conversion and as the default when adding AI seats.
    pub async fn find_default_ai_profile(
        &self,
        txn: &DatabaseTransaction,
    ) -> Result<ai_profiles::AiProfile, AppError> {
        use crate::ai::registry;
        let default = registry::default_ai().ok_or_else(|| {
            AppError::internal(
                crate::errors::ErrorCode::InternalError,
                format!(
                    "DEFAULT_AI_NAME '{}' is not registered",
                    registry::DEFAULT_AI_NAME
                ),
                std::io::Error::other("default AI not registered"),
            )
        })?;
        let profile = ai_profiles::find_by_registry_variant(
            txn,
            default.name,
            default.version,
            "default",
        )
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| {
            AppError::internal(
                crate::errors::ErrorCode::InternalError,
                format!(
                    "Default AI profile not found (registry_name: {}, version: {}, variant: default)",
                    default.name,
                    default.version
                ),
                std::io::Error::other("AI profile not found"),
            )
        })?;
        Ok(profile)
    }
}
