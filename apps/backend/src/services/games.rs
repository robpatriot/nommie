//! Game state loading and construction services.

use sea_orm::{ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder};

use crate::adapters::games_sea::GameCreate;
use crate::domain::cards_parsing::from_stored_format;
use crate::domain::state::{GameState, Phase, PreviousRound, RoundState};
use crate::domain::Card;
use crate::entities::game_players;
use crate::entities::games::{self, GameState as DbGameState, GameVisibility};
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::games::Game;
use crate::repos::memberships::GameRole;
use crate::repos::{
    ai_overrides, bids, games as games_repo, hands, memberships, plays, rounds, scores, tricks,
};

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
                    round_no: 0,
                    hand_size: 0,
                    hands: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
                    turn_start: 0,
                    turn: 0,
                    leader: 0,
                    trick_no: 0,
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

                let prev_hand_size = prev_round.hand_size;

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

        // 12. Determine turn_start, turn, leader
        let dealer_pos = game.dealer_pos().unwrap_or(0);
        let turn_start = (dealer_pos + 1) % 4;

        let last_completed_trick_winner: Option<u8> = all_tricks
            .iter()
            .rev()
            .find(|t| (0..=3).contains(&t.winner_seat))
            .map(|t| t.winner_seat);

        let leader = match phase {
            Phase::Trick { .. } => {
                if let Some((seat, _)) = trick_plays.first() {
                    *seat
                } else if let Some(winner) = last_completed_trick_winner {
                    winner
                } else {
                    turn_start
                }
            }
            _ => last_completed_trick_winner.unwrap_or(turn_start),
        };

        let turn = match phase {
            Phase::Bidding => {
                let bid_count = all_bids.len() as u8;
                (turn_start + bid_count) % 4
            }
            Phase::TrumpSelect => winning_bidder.unwrap_or(turn_start),
            Phase::Trick { .. } => {
                let plays_count = trick_plays.len() as u8;
                (leader + plays_count) % 4
            }
            _ => turn_start,
        };

        Ok(GameState {
            phase,
            round_no: current_round_no,
            hand_size,
            hands: hands_array,
            turn_start,
            turn,
            leader,
            trick_no: current_trick_no,
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
            0,     // turn_order: 0 for creator
            false, // is_ready: false
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

    /// Find the most recently active game for a user (based on game.updated_at).
    ///
    /// Returns the game ID of the game where the user is a member that has the most
    /// recent activity (highest updated_at timestamp).
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `user_id` - ID of the user
    ///
    /// # Returns
    /// Option<i64> - Game ID if found, None if user has no games
    pub async fn find_last_active_game(
        &self,
        txn: &DatabaseTransaction,
        user_id: i64,
    ) -> Result<Option<i64>, AppError> {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

        // Find all games where user is a member, ordered by updated_at DESC
        let memberships = game_players::Entity::find()
            .filter(game_players::Column::PlayerKind.eq(game_players::PlayerKind::Human))
            .filter(game_players::Column::HumanUserId.eq(user_id))
            .all(txn)
            .await
            .map_err(AppError::from)?;

        if memberships.is_empty() {
            return Ok(None);
        }

        // Get game IDs from memberships
        let game_ids: Vec<i64> = memberships.iter().map(|m| m.game_id).collect();

        // Find the game with the most recent updated_at
        let last_active_game = games::Entity::find()
            .filter(games::Column::Id.is_in(game_ids))
            .order_by_desc(games::Column::UpdatedAt)
            .one(txn)
            .await
            .map_err(AppError::from)?;

        Ok(last_active_game.map(|game| game.id))
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
            memberships.iter().map(|m| m.turn_order).collect();

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

    /// Join a user to a game.
    ///
    /// This method handles:
    /// - Validating game exists and is in LOBBY state
    /// - Checking user is not already a member
    /// - Checking capacity, replacing lowest-seat AI if needed
    /// - Finding next available turn_order (when seats are free)
    /// - Creating membership
    /// - Returning updated game and memberships
    ///
    /// All operations are performed within the provided transaction.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `game_id` - ID of the game to join
    /// * `user_id` - ID of the user joining
    ///
    /// # Returns
    /// Tuple of (game entity model, all memberships)
    pub async fn join_game(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        user_id: i64,
    ) -> Result<(Game, Vec<memberships::GameMembership>), AppError> {
        // Fetch game and verify it exists
        let game = games_repo::find_by_id(txn, game_id)
            .await
            .map_err(|e| {
                AppError::internal(
                    crate::errors::ErrorCode::InternalError,
                    format!("failed to fetch game: {e}"),
                    e,
                )
            })?
            .ok_or_else(|| {
                AppError::not_found(
                    crate::errors::ErrorCode::GameNotFound,
                    format!("Game {game_id} not found"),
                )
            })?;

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
                detail: format!("User {} is already a member of game {}", user_id, game_id),
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
                .min_by_key(|m| m.turn_order)
                .cloned()
                .ok_or_else(|| {
                    AppError::internal(
                        crate::errors::ErrorCode::InternalError,
                        "Failed to select AI seat for replacement".to_string(),
                        std::io::Error::other("no AI seat found despite non-empty list"),
                    )
                })?;

            // Remove any overrides and the AI membership itself
            ai_overrides::delete_by_game_player_id(txn, ai_to_replace.id)
                .await
                .map_err(AppError::from)?;
            memberships::delete_membership(txn, ai_to_replace.id)
                .await
                .map_err(AppError::from)?;

            // Create human membership in the freed seat
            memberships::create_membership(
                txn,
                game_id,
                user_id,
                ai_to_replace.turn_order,
                false,
                GameRole::Player,
            )
            .await
            .map_err(AppError::from)?;
        } else {
            // Seats available: find next free seat and join there
            let next_turn_order = self
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
                next_turn_order,
                false,
                GameRole::Player,
            )
            .await
            .map_err(AppError::from)?;
        }

        // Fetch updated memberships and reload game
        let updated_memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        let game = games_repo::require_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        Ok((game, updated_memberships))
    }
}
