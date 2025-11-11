//! Game state loading and construction services.

use sea_orm::DatabaseTransaction;

use crate::adapters::games_sea::{self, GameCreate};
use crate::domain::cards_parsing::from_stored_format;
use crate::domain::state::{GameState, Phase, RoundState};
use crate::domain::{Card, Suit, Trump};
use crate::entities::games::{self, GameState as DbGameState, GameVisibility};
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::memberships::GameRole;
use crate::repos::{bids, games as games_repo, hands, memberships, plays, rounds, scores, tricks};
use crate::utils::join_code::generate_join_code;

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
        let game = games_sea::require_game(txn, game_id).await?;

        // 2. Check if game has started (has a current round)
        let current_round_no = match game.current_round {
            Some(round_no) => round_no,
            None => {
                // Game hasn't started yet - return empty initial state (no rounds exist)
                let phase = match game.state {
                    DbGameState::Lobby => Phase::Init,
                    DbGameState::Dealing => Phase::Init,
                    DbGameState::Bidding => Phase::Bidding,
                    DbGameState::TrumpSelection => Phase::TrumpSelect,
                    _ => Phase::Init,
                };

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
        })? as u8;

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
            if hand.player_seat >= 0 && hand.player_seat < 4 {
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
            if bid.player_seat >= 0 && bid.player_seat < 4 {
                bids_array[bid.player_seat as usize] = Some(bid.bid_value as u8);
            }
        }

        // 6. Determine winning bidder
        let winning_bidder = if bids_array.iter().all(|b| b.is_some()) {
            bids::find_winning_bid(txn, round.id)
                .await?
                .map(|b| b.player_seat as u8)
        } else {
            None
        };

        // 7. Load trump
        let trump = round.trump.map(|t| match t {
            rounds::Trump::Clubs => Trump::Clubs,
            rounds::Trump::Diamonds => Trump::Diamonds,
            rounds::Trump::Hearts => Trump::Hearts,
            rounds::Trump::Spades => Trump::Spades,
            rounds::Trump::NoTrump => Trump::NoTrump,
        });

        // 8. Count tricks won
        let all_tricks = tricks::find_all_by_round(txn, round.id).await?;
        let mut tricks_won = [0u8; 4];
        for trick in &all_tricks {
            let winner = trick.winner_seat;
            if !(0..=3).contains(&winner) {
                continue;
            }

            // Guard against placeholder winners for the in-progress trick (legacy data used 0)
            if game.state == DbGameState::TrickPlay
                && trick.trick_no == game.current_trick_no
                && winner == 0
            {
                continue;
            }

            tricks_won[winner as usize] += 1;
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
                        Ok((p.player_seat as u8, card))
                    })
                    .collect::<Result<Vec<_>, DomainError>>()?;

                let lead = match current_trick.lead_suit {
                    tricks::Suit::Clubs => Suit::Clubs,
                    tricks::Suit::Diamonds => Suit::Diamonds,
                    tricks::Suit::Hearts => Suit::Hearts,
                    tricks::Suit::Spades => Suit::Spades,
                };

                (plays, Some(lead))
            } else {
                (Vec::new(), None)
            }
        } else {
            (Vec::new(), None)
        };

        // 10. Load cumulative scores
        let scores_total = scores::get_current_totals(txn, game_id).await?;

        // 11. Convert DB phase to domain Phase
        let phase = match game.state {
            DbGameState::Lobby => Phase::Init,
            DbGameState::Dealing => Phase::Init,
            DbGameState::Bidding => Phase::Bidding,
            DbGameState::TrumpSelection => Phase::TrumpSelect,
            DbGameState::TrickPlay => Phase::Trick {
                trick_no: current_trick_no as u8,
            },
            DbGameState::Scoring => Phase::Scoring,
            DbGameState::BetweenRounds => Phase::Complete,
            DbGameState::Completed => Phase::GameOver,
            DbGameState::Abandoned => Phase::GameOver,
        };

        // 12. Determine turn_start, turn, leader
        let dealer_pos = game.dealer_pos().unwrap_or(0) as u8;
        let turn_start = (dealer_pos + 1) % 4;

        let last_completed_trick_winner: Option<u8> = all_tricks
            .iter()
            .rev()
            .find(|t| (0..=3).contains(&t.winner_seat))
            .map(|t| t.winner_seat as u8);

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
            round_no: current_round_no as u8,
            hand_size,
            hands: hands_array,
            turn_start,
            turn,
            leader,
            trick_no: current_trick_no as u8,
            scores_total,
            round: RoundState {
                trick_plays,
                trick_lead,
                tricks_won,
                trump,
                bids: bids_array,
                winning_bidder,
            },
        })
    }

    /// Create a game with the creator as the first member.
    ///
    /// This method handles:
    /// - Generating a unique join code (with retry logic)
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
    ) -> Result<(games::Model, Vec<memberships::GameMembership>), AppError> {
        const MAX_RETRIES: usize = 5;

        // Generate join code and create game with retry logic.
        // The database unique constraint on `games.join_code` ensures uniqueness atomically.
        // If a conflict occurs (race condition), we generate a new code and retry.
        for attempt in 0..MAX_RETRIES {
            // Generate a random join code
            let join_code = generate_join_code();

            // Try to create game with this code
            let dto = GameCreate::new(&join_code)
                .with_visibility(GameVisibility::Public)
                .by(user_id);

            let dto = if let Some(ref n) = name {
                dto.with_name(n.clone())
            } else {
                dto
            };

            match games_repo::create_game(txn, dto)
                .await
                .map_err(AppError::from)
            {
                Ok(game) => {
                    // Successfully created game, add creator as first member
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

                    // Fetch game entity and all memberships
                    let game_model = games_sea::find_by_id(txn, game.id)
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
                                format!("Game {} not found after creation", game.id),
                            )
                        })?;

                    let all_memberships = memberships::find_all_by_game(txn, game.id)
                        .await
                        .map_err(AppError::from)?;

                    return Ok((game_model, all_memberships));
                }
                Err(AppError::Conflict {
                    code: crate::errors::ErrorCode::JoinCodeConflict,
                    ..
                }) => {
                    // Join code conflict (unique constraint violation), retry with new code
                    if attempt == MAX_RETRIES - 1 {
                        return Err(AppError::internal(
                            crate::errors::ErrorCode::InternalError,
                            "Failed to create game: maximum retries exceeded for join code conflicts".to_string(),
                            std::io::Error::other("join code generation retry limit exceeded"),
                        ));
                    }
                    continue;
                }
                Err(e) => {
                    // Other error, return immediately
                    return Err(e);
                }
            }
        }

        // Should never reach here, but just in case
        Err(AppError::internal(
            crate::errors::ErrorCode::InternalError,
            "Failed to create game: maximum retries exceeded".to_string(),
            std::io::Error::other("join code generation retry limit exceeded"),
        ))
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
    ) -> Option<i32> {
        let used_turn_orders: std::collections::HashSet<i32> =
            memberships.iter().map(|m| m.turn_order).collect();

        (0..4).find(|&order| !used_turn_orders.contains(&order))
    }

    /// Join a user to a game.
    ///
    /// This method handles:
    /// - Validating game exists and is in LOBBY state
    /// - Checking user is not already a member
    /// - Checking game is not full (max 4 players)
    /// - Finding next available turn_order
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
    ) -> Result<(games::Model, Vec<memberships::GameMembership>), AppError> {
        // Fetch game and verify it exists
        let game_model = games_sea::find_by_id(txn, game_id)
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
                    format!("Game with ID {game_id} not found"),
                )
            })?;

        // Verify game is in LOBBY state
        if game_model.state != DbGameState::Lobby {
            return Err(AppError::bad_request(
                crate::errors::ErrorCode::PhaseMismatch,
                format!(
                    "Game is not in LOBBY state (current state: {:?})",
                    game_model.state
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

        let player_count = all_memberships
            .iter()
            .filter(|m| m.role == GameRole::Player)
            .count();

        // Check if game is full
        if player_count >= 4 {
            return Err(AppError::bad_request(
                crate::errors::ErrorCode::ValidationError,
                "Game is full (maximum 4 players)".to_string(),
            ));
        }

        // Find next available turn_order
        let next_turn_order = self
            .find_next_available_seat(&all_memberships)
            .ok_or_else(|| {
                AppError::internal(
                    crate::errors::ErrorCode::InternalError,
                    "No available turn order found".to_string(),
                    std::io::Error::other("turn order calculation failed"),
                )
            })?;

        // Create membership
        memberships::create_membership(
            txn,
            game_id,
            user_id,
            next_turn_order,
            false, // is_ready: false
            GameRole::Player,
        )
        .await
        .map_err(AppError::from)?;

        // Fetch updated memberships
        let updated_memberships = memberships::find_all_by_game(txn, game_id)
            .await
            .map_err(AppError::from)?;

        Ok((game_model, updated_memberships))
    }
}
