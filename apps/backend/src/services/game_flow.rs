//! Game flow orchestration service - bridges pure domain logic with DB persistence.
//!
//! This service provides fine-grained transition methods for game state progression
//! and a test/bot helper that composes them into a happy path.

use sea_orm::DatabaseTransaction;
use tracing::{debug, info};

use crate::adapters::games_sea::{self, GameUpdateRound, GameUpdateState};
use crate::domain::bidding::Bid;
use crate::domain::cards_parsing::from_stored_format;
use crate::domain::{card_beats, deal_hands, hand_size_for_round, Card, Suit};
use crate::entities::games::GameState as DbGameState;
use crate::error::AppError;
use crate::errors::domain::{DomainError, ValidationKind};
use crate::repos::{bids, hands, plays, rounds, scores, tricks};

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
    pub async fn deal_round(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<(), AppError> {
        info!(game_id, "Dealing new round");

        // Load game from DB
        let game = games_sea::require_game(txn, game_id).await?;

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
        let dealt_hands = deal_hands(4, hand_size, seed)?;

        // Update DB: state and round number
        let update_state = GameUpdateState::new(game_id, DbGameState::Bidding, game.lock_version);
        let updated_game = games_sea::update_state(txn, update_state).await?;

        // On first round, set starting_dealer_pos (defaults to 0)
        let mut update_round = GameUpdateRound::new(game_id, updated_game.lock_version)
            .with_current_round(next_round as i16);

        if next_round == 1 {
            // Initialize starting dealer position on first round
            let starting_dealer = 0; // Could be randomized or determined by game rules
            update_round = update_round.with_starting_dealer_pos(starting_dealer);
        }

        let updated_game = games_sea::update_round(txn, update_round).await?;

        // Compute current dealer (hand_size and dealer_pos are now computed)
        let computed_hand_size = updated_game.hand_size().unwrap_or(0);
        let computed_dealer_pos = updated_game.dealer_pos().unwrap_or(0);

        // Create round record in DB
        let round = rounds::create_round(
            txn,
            game_id,
            next_round as i16,
            computed_hand_size,
            computed_dealer_pos,
        )
        .await?;

        // Persist dealt hands to DB
        let hands_to_store: Vec<(i16, Vec<hands::Card>)> = dealt_hands
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
                (idx as i16, cards)
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

    /// Submit a bid for a player in the current round.
    ///
    /// Public method that records the bid and processes game state (transitions + AI).
    pub async fn submit_bid(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        bid_value: u8,
    ) -> Result<(), AppError> {
        self.submit_bid_internal(txn, game_id, player_seat, bid_value)
            .await?;
        self.process_game_state(txn, game_id).await?;
        Ok(())
    }

    /// Internal bid submission - just records the bid without processing.
    ///
    /// Used by AI loop to avoid recursion. Handlers should use submit_bid() instead.
    async fn submit_bid_internal(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        bid_value: u8,
    ) -> Result<(), AppError> {
        debug!(game_id, player_seat, bid_value, "Submitting bid");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

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

        // Find the current round
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

        // Determine bid_order (how many bids have been placed already)
        let bid_order = bids::count_bids_by_round(txn, round.id).await? as i16;

        // Turn order validation: bidding starts at dealer+1, then proceeds clockwise
        let dealer_pos = game.dealer_pos().ok_or_else(|| {
            DomainError::validation(
                ValidationKind::Other("NO_DEALER".into()),
                "Dealer position not set",
            )
        })?;

        let expected_seat = (dealer_pos + 1 + bid_order) % 4;
        if player_seat != expected_seat {
            return Err(DomainError::validation(
                ValidationKind::OutOfTurn,
                format!(
                    "Not your turn to bid. Expected player {expected_seat} (seat {expected_seat}), got player {player_seat}"
                ),
            )
            .into());
        }

        // Dealer bid restriction: if this is the 4th (final) bid, check dealer rule
        if bid_order == 3 {
            // This is the dealer's bid - sum of all bids cannot equal hand_size
            let existing_bids = bids::find_all_by_round(txn, round.id).await?;
            let existing_sum: i16 = existing_bids.iter().map(|b| b.bid_value).sum();
            let proposed_sum = existing_sum + bid_value as i16;

            if proposed_sum == hand_size as i16 {
                return Err(DomainError::validation(
                    ValidationKind::InvalidBid,
                    format!(
                        "Dealer cannot bid {bid_value}: sum would be {proposed_sum} = hand_size"
                    ),
                )
                .into());
            }
        }

        // Persist the bid
        bids::create_bid(txn, round.id, player_seat, bid_value as i16, bid_order).await?;

        info!(
            game_id,
            player_seat, bid_value, bid_order, "Bid persisted successfully"
        );

        // Bump lock_version on game to reflect bid state change
        // This ensures each bid increments the version, not just state transitions
        let updated_game = games_sea::require_game(txn, game_id).await?;
        let lock_bump = GameUpdateRound::new(game_id, updated_game.lock_version);
        games_sea::update_round(txn, lock_bump).await?;

        Ok(())
    }

    /// Set trump for the current round.
    ///
    /// Public method that sets trump and processes game state (transitions + AI).
    pub async fn set_trump(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        trump: rounds::Trump,
    ) -> Result<(), AppError> {
        self.set_trump_internal(txn, game_id, player_seat, trump)
            .await?;
        self.process_game_state(txn, game_id).await?;
        Ok(())
    }

    /// Internal trump setting - just sets trump without processing.
    ///
    /// Used by AI loop to avoid recursion. Handlers should use set_trump() instead.
    async fn set_trump_internal(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        trump: rounds::Trump,
    ) -> Result<(), AppError> {
        info!(game_id, player_seat, trump = ?trump, "Setting trump");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

        if game.state != DbGameState::TrumpSelection {
            return Err(DomainError::validation(
                ValidationKind::PhaseMismatch,
                "Not in trump selection phase",
            )
            .into());
        }

        // Get current round
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

        // Determine winning bidder and validate
        let winning_bid = bids::find_winning_bid(txn, round.id)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("NO_WINNING_BID".into()),
                    "No winning bid found",
                )
            })?;

        if winning_bid.player_seat != player_seat {
            return Err(DomainError::validation(
                ValidationKind::OutOfTurn,
                format!(
                    "Only the winning bidder (seat {}) can choose trump, not seat {}",
                    winning_bid.player_seat, player_seat
                ),
            )
            .into());
        }

        // Set trump on the round
        rounds::update_trump(txn, round.id, trump).await?;

        info!(
            game_id,
            player_seat,
            trump = ?trump,
            "Trump set by winning bidder"
        );

        Ok(())
    }

    /// Play a card for a player in the current trick.
    ///
    /// Public method that records the card play and processes game state (transitions + AI).
    pub async fn play_card(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        card: Card,
    ) -> Result<(), AppError> {
        self.play_card_internal(txn, game_id, player_seat, card)
            .await?;
        self.process_game_state(txn, game_id).await?;
        Ok(())
    }

    /// Internal card play - just records the play without processing.
    ///
    /// Used by AI loop to avoid recursion. Handlers should use play_card() instead.
    async fn play_card_internal(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        card: Card,
    ) -> Result<(), AppError> {
        debug!(game_id, player_seat, "Playing card");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

        if game.state != DbGameState::TrickPlay {
            return Err(DomainError::validation(
                ValidationKind::PhaseMismatch,
                "Not in trick play phase",
            )
            .into());
        }

        // Get current round
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

        // SECURITY: Validate the card is in the player's remaining hand
        // This prevents cheating by playing cards they don't have or already played
        use crate::domain::player_view::CurrentRoundInfo;
        let player_state = CurrentRoundInfo::load(txn, game_id, player_seat).await?;

        if !player_state.hand.contains(&card) {
            return Err(DomainError::validation(
                ValidationKind::CardNotInHand,
                format!(
                    "Card {:?} of {:?} is not in player's hand",
                    card.rank, card.suit
                ),
            )
            .into());
        }

        // Also validate it's a legal play (suit following rules)
        let legal_plays = player_state.legal_plays()?;
        if !legal_plays.contains(&card) {
            return Err(DomainError::validation(
                ValidationKind::MustFollowSuit,
                "Must follow suit if possible",
            )
            .into());
        }

        // Get current trick number
        let current_trick_no = game.current_trick_no;

        // Find or create the current trick
        // Lead suit is determined from the first play.
        // Winner is set to 0 initially and determined by resolve_trick() after all 4 plays.
        let trick = if let Some(existing) =
            tricks::find_by_round_and_trick(txn, round.id, current_trick_no).await?
        {
            existing
        } else {
            // First play in this trick - create trick record
            // Use card's suit as lead suit
            let lead_suit = match card.suit {
                crate::domain::Suit::Clubs => tricks::Suit::Clubs,
                crate::domain::Suit::Diamonds => tricks::Suit::Diamonds,
                crate::domain::Suit::Hearts => tricks::Suit::Hearts,
                crate::domain::Suit::Spades => tricks::Suit::Spades,
            };

            // Winner placeholder - will be determined by resolve_trick() after 4th play
            tricks::create_trick(txn, round.id, current_trick_no, lead_suit, 0).await?
        };

        // Determine play_order (how many plays already in this trick)
        let play_order = plays::count_plays_by_trick(txn, trick.id).await? as i16;

        // Convert domain Card to repo Card
        let card_for_storage = plays::Card {
            suit: format!("{:?}", card.suit).to_uppercase(),
            rank: format!("{:?}", card.rank).to_uppercase(),
        };

        // Persist the play
        plays::create_play(txn, trick.id, player_seat, card_for_storage, play_order).await?;

        info!(
            game_id,
            player_seat,
            trick_no = current_trick_no,
            play_order,
            "Card play persisted successfully"
        );

        // Bump lock_version on game to reflect card play state change
        // This ensures each card play increments the version (consistent with bid behavior)
        let updated_game = games_sea::require_game(txn, game_id).await?;
        let lock_bump = GameUpdateRound::new(game_id, updated_game.lock_version);
        games_sea::update_round(txn, lock_bump).await?;

        Ok(())
    }

    /// Resolve a completed trick: determine winner and advance to next trick.
    ///
    /// Loads the 4 plays, uses domain logic to determine winner based on trump/lead,
    /// updates the trick with winner, and advances current_trick_no or transitions to Scoring.
    pub async fn resolve_trick(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<(), AppError> {
        debug!(game_id, "Resolving trick");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

        if game.state != DbGameState::TrickPlay {
            return Err(DomainError::validation(
                ValidationKind::PhaseMismatch,
                "Not in trick play phase",
            )
            .into());
        }

        // Get current round and trick
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

        let current_trick_no = game.current_trick_no;
        let hand_size = game.hand_size().ok_or_else(|| {
            DomainError::validation(ValidationKind::InvalidHandSize, "Hand size not set")
        })?;

        // Verify trick exists and has 4 plays
        let trick = tricks::find_by_round_and_trick(txn, round.id, current_trick_no)
            .await?
            .ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("TRICK_NOT_FOUND".into()),
                    "Trick not found",
                )
            })?;

        let all_plays = plays::find_all_by_trick(txn, trick.id).await?;
        if all_plays.len() != 4 {
            return Err(DomainError::validation(
                ValidationKind::Other("INCOMPLETE_TRICK".into()),
                format!("Trick has {} plays, need 4", all_plays.len()),
            )
            .into());
        }

        // Trump must be set by this point
        let trump_domain = round
            .trump
            .ok_or_else(|| {
                DomainError::validation(ValidationKind::Other("NO_TRUMP".into()), "Trump not set")
            })
            .map(|t| match t {
                rounds::Trump::Clubs => crate::domain::Trump::Clubs,
                rounds::Trump::Diamonds => crate::domain::Trump::Diamonds,
                rounds::Trump::Hearts => crate::domain::Trump::Hearts,
                rounds::Trump::Spades => crate::domain::Trump::Spades,
                rounds::Trump::NoTrump => crate::domain::Trump::NoTrump,
            })?;

        // Determine winner using domain logic
        let lead_suit_domain = match trick.lead_suit {
            tricks::Suit::Clubs => Suit::Clubs,
            tricks::Suit::Diamonds => Suit::Diamonds,
            tricks::Suit::Hearts => Suit::Hearts,
            tricks::Suit::Spades => Suit::Spades,
        };

        // Parse all plays into domain cards and determine winner
        let mut winner_seat = all_plays[0].player_seat;
        let mut winner_card = from_stored_format(&all_plays[0].card.suit, &all_plays[0].card.rank)?;

        // Compare each subsequent card to the current winner
        for play in &all_plays[1..] {
            let challenger_card = from_stored_format(&play.card.suit, &play.card.rank)?;

            // Check if challenger beats current winner
            if card_beats(challenger_card, winner_card, lead_suit_domain, trump_domain) {
                winner_seat = play.player_seat;
                winner_card = challenger_card; // Update winner card
            }
        }

        info!(
            game_id,
            trick_no = current_trick_no,
            winner_seat,
            "Trick winner determined"
        );

        // Update trick with winner
        tricks::update_winner(txn, trick.id, winner_seat).await?;

        // Advance to next trick or Scoring phase
        let next_trick_no = current_trick_no + 1;
        if next_trick_no >= hand_size {
            // All tricks complete - transition to Scoring
            info!(
                game_id,
                trick_no = current_trick_no,
                "All tricks complete, transitioning to Scoring"
            );
            let update = GameUpdateState::new(game_id, DbGameState::Scoring, game.lock_version);
            games_sea::update_state(txn, update).await?;
        } else {
            // Advance to next trick
            let update = GameUpdateRound::new(game_id, game.lock_version)
                .with_current_trick_no(next_trick_no);
            games_sea::update_round(txn, update).await?;
            info!(
                game_id,
                trick_no = current_trick_no,
                next_trick_no,
                winner_seat,
                "Trick resolved, advanced to next trick"
            );
        }

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
        info!(game_id, "Scoring round");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

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
        let mut tricks_won = [0i16; 4];
        for trick in &all_tricks {
            if trick.winner_seat >= 0 && trick.winner_seat < 4 {
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
                if score.player_seat >= 0 && score.player_seat < 4 {
                    totals[score.player_seat as usize] = score.total_score_after;
                }
            }
            totals
        } else {
            [0, 0, 0, 0]
        };

        // Calculate and persist scores for all 4 players
        for seat in 0..4 {
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
            let total_after = previous_totals[seat as usize] + round_score;

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
        let update = GameUpdateState::new(game_id, DbGameState::Scoring, game.lock_version);
        games_sea::update_state(txn, update).await?;

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
        info!(game_id, "Advancing to next round");

        // Load game
        let game = games_sea::require_game(txn, game_id).await?;

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
            games_sea::update_state(txn, update).await?;
            info!(game_id, rounds_played = current_round, "Game completed");
            debug!(game_id, "Transition: Scoring -> Completed");
        } else {
            // More rounds to play - transition to BetweenRounds and reset trick counter
            let update =
                GameUpdateState::new(game_id, DbGameState::BetweenRounds, game.lock_version);
            let updated_game = games_sea::update_state(txn, update).await?;

            // Reset current_trick_no for next round
            let reset_trick =
                GameUpdateRound::new(game_id, updated_game.lock_version).with_current_trick_no(0);
            games_sea::update_round(txn, reset_trick).await?;

            info!(game_id, current_round, "Advanced to BetweenRounds");
            debug!(game_id, "Transition: Scoring -> BetweenRounds");
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
        use crate::adapters::memberships_sea;
        use crate::repos::memberships;

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

        // Mark ready
        let dto = memberships_sea::MembershipSetReady {
            id: membership.id,
            is_ready: true,
        };
        memberships_sea::set_membership_ready(txn, dto).await?;

        info!(game_id, user_id, "Player marked ready");

        // Check if all players are ready
        let all_memberships = memberships::find_all_by_game(txn, game_id).await?;
        let all_ready = all_memberships.iter().all(|m| m.is_ready);

        if all_ready && all_memberships.len() == 4 {
            info!(game_id, "All players ready, starting game");
            // Deal first round
            self.deal_round(txn, game_id).await?;
            // Process game state to handle transitions and AI actions
            self.process_game_state(txn, game_id).await?;
        }

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
    /// Performance: Maintains RoundContext cache across iterations within the same round,
    /// only reloading when the round number changes. This avoids ~1,400 redundant cache
    /// creations per game (reduces to ~26, once per round).
    pub async fn process_game_state(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
    ) -> Result<(), AppError> {
        use crate::entities::games::GameState as DbGameState;
        use crate::services::round_context::RoundContext;

        const MAX_ITERATIONS: usize = 2000; // Allow for full 26-round game with all actions

        // Cache that persists across iterations within the same round
        let mut cached_round: Option<(i16, RoundContext)> = None;

        for _iteration in 0..MAX_ITERATIONS {
            let game = games_sea::require_game(txn, game_id).await?;

            // Priority 1: Check if we need a state transition
            if self.check_and_apply_transition_internal(txn, &game).await? {
                // Transition happened, loop again
                continue;
            }

            // Check if we need cache and if it needs refreshing
            let needs_cache = matches!(
                game.state,
                DbGameState::Bidding | DbGameState::TrumpSelection | DbGameState::TrickPlay
            );

            if needs_cache {
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
                            "Creating RoundContext cache for new round"
                        );
                        let context = RoundContext::load(txn, game_id, current_round).await?;
                        cached_round = Some((current_round, context));
                    }
                    // else: reuse existing cache (optimization!)
                }
            } else {
                // Not in a state that benefits from caching
                if cached_round.is_some() {
                    debug!(game_id, "Clearing RoundContext cache (exited round states)");
                    cached_round = None;
                }
            }

            // Priority 2: Check if an AI needs to act (pass cache if available)
            if self
                .check_and_execute_ai_action_with_cache(
                    txn,
                    &game,
                    cached_round.as_ref().map(|(_, ctx)| ctx),
                )
                .await?
            {
                // AI acted, loop again (cache remains valid for next iteration!)
                continue;
            }

            // Nothing to do - we're done
            return Ok(());
        }

        Err(AppError::internal(format!(
            "process_game_state exceeded max iterations {MAX_ITERATIONS}"
        )))
    }

    /// Check if current game state requires a transition and apply it.
    ///
    /// Returns true if a transition was applied.
    /// Does NOT call process_game_state - the caller loops instead.
    async fn check_and_apply_transition_internal(
        &self,
        txn: &DatabaseTransaction,
        game: &crate::entities::games::Model,
    ) -> Result<bool, AppError> {
        use crate::entities::games::GameState as DbGameState;

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
                    let update = GameUpdateState::new(
                        game.id,
                        DbGameState::TrumpSelection,
                        game.lock_version,
                    );
                    games_sea::update_state(txn, update).await?;
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
                    // Trump is set - transition to TrickPlay
                    let updated_game = games_sea::require_game(txn, game.id).await?;
                    let update = GameUpdateState::new(
                        game.id,
                        DbGameState::TrickPlay,
                        updated_game.lock_version,
                    );
                    games_sea::update_state(txn, update).await?;
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
                        self.resolve_trick(txn, game.id).await?;
                        // Note: resolve_trick will call process_game_state
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
                    self.advance_to_next_round(txn, game.id).await?;
                    // Note: advance_to_next_round will call process_game_state
                    return Ok(true);
                } else {
                    // Need to score the round first
                    self.score_round(txn, game.id).await?;
                    // Note: score_round will call process_game_state
                    return Ok(true);
                }
            }
            DbGameState::BetweenRounds => {
                // Automatically deal next round
                self.deal_round(txn, game.id).await?;
                // Note: deal_round will call process_game_state
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

    /// Check if an AI player needs to act and execute the action (with optional cache).
    ///
    /// Returns true if an AI action was executed (which will trigger recursive processing).
    ///
    /// If round_context is provided, uses it to avoid redundant database queries.
    /// Otherwise, loads player/AI data from database (slower fallback).
    async fn check_and_execute_ai_action_with_cache(
        &self,
        txn: &DatabaseTransaction,
        game: &crate::entities::games::Model,
        round_context: Option<&crate::services::round_context::RoundContext>,
    ) -> Result<bool, AppError> {
        use sea_orm::{EntityTrait, JoinType, QuerySelect, RelationTrait};

        use crate::ai::create_ai;
        use crate::entities::ai_profiles;
        use crate::repos::memberships;

        // Determine whose turn it is
        let action_info = self.determine_next_action(txn, game).await?;

        let Some((player_seat, action_type)) = action_info else {
            return Ok(false); // No action needed
        };

        // Check if this player is an AI (use cache if available)
        let (user_id, profile) = if let Some(ctx) = round_context {
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
                .find(|m| m.turn_order == player_seat as i32);

            let Some(player) = player else {
                // Player not found at this seat - stop AI processing
                debug!(
                    game.id,
                    player_seat, "No player at seat, stopping AI processing"
                );
                return Ok(false);
            };

            let profile = ctx.get_ai_profile(player.user_id);

            if profile.is_none() {
                debug!(
                    game.id,
                    player_seat, "Human player's turn, stopping AI processing"
                );
                return Ok(false);
            }

            (player.user_id, profile.cloned())
        } else {
            // Slow path: Load from database (used for Lobby, Dealing, etc.)
            let memberships = memberships::find_all_by_game(txn, game.id).await?;
            let Some(player) = memberships
                .iter()
                .find(|m| m.turn_order == player_seat as i32)
            else {
                debug!(
                    game.id,
                    player_seat, "No player found at seat, stopping AI processing"
                );
                return Ok(false);
            };

            let ai_info = crate::entities::users::Entity::find_by_id(player.user_id)
                .join(
                    JoinType::LeftJoin,
                    crate::entities::users::Relation::AiProfiles.def(),
                )
                .select_also(ai_profiles::Entity)
                .one(txn)
                .await?;

            let Some((user, maybe_profile)) = ai_info else {
                return Err(DomainError::validation(
                    ValidationKind::Other("USER_NOT_FOUND".into()),
                    "User not found",
                )
                .into());
            };

            if !user.is_ai {
                debug!(
                    game.id,
                    player_seat, "Human player's turn, stopping AI processing"
                );
                return Ok(false);
            }

            let profile = maybe_profile.ok_or_else(|| {
                DomainError::validation(
                    ValidationKind::Other("AI_PROFILE_NOT_FOUND".into()),
                    format!("AI profile not found for user {}", user.id),
                )
            })?;

            (user.id, Some(profile))
        };

        let profile = profile
            .ok_or_else(|| AppError::internal(format!("AI profile missing for user {user_id}")))?;

        info!(game.id, player_seat, action = ?action_type, "Processing AI turn");

        // Create AI player
        let ai_type = profile.playstyle.as_deref().unwrap_or("random");
        let ai = create_ai(ai_type, profile.config.as_ref())
            .ok_or_else(|| AppError::internal(format!("Unknown AI type: {ai_type}")))?;

        // Execute action with retries, using cache if available
        const MAX_RETRIES_PER_ACTION: usize = 3;
        let mut last_error = None;

        for retry in 0..MAX_RETRIES_PER_ACTION {
            match self
                .execute_ai_action_with_cache(
                    txn,
                    game.id,
                    player_seat,
                    action_type,
                    ai.as_ref(),
                    round_context,
                )
                .await
            {
                Ok(()) => {
                    info!(
                        game.id,
                        player_seat,
                        action = ?action_type,
                        retry,
                        cached = round_context.is_some(),
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
        Err(last_error
            .unwrap_or_else(|| AppError::internal("AI action failed with no error details")))
    }

    /// Determine what action is needed next and whose turn it is.
    ///
    /// Returns None if no action is needed (game complete or waiting).
    /// Returns Some((seat, action_type)) if an action is needed.
    async fn determine_next_action(
        &self,
        txn: &DatabaseTransaction,
        game: &crate::entities::games::Model,
    ) -> Result<Option<(i16, ActionType)>, AppError> {
        use crate::entities::games::GameState as DbGameState;
        use crate::repos::{bids, tricks};

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

                let bid_count = bids::count_bids_by_round(txn, round.id).await? as i16;
                if bid_count >= 4 {
                    // All bids placed - no action needed (state transition will happen)
                    Ok(None)
                } else {
                    let dealer_pos = game.dealer_pos().unwrap_or(0);
                    let next_seat = (dealer_pos + 1 + bid_count) % 4;
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
                    let play_count = plays::count_plays_by_trick(txn, trick.id).await? as i16;
                    let all_plays = plays::find_all_by_trick(txn, trick.id).await?;
                    let first_player = all_plays.first().map(|p| p.player_seat).unwrap_or(0);
                    let next_seat = (first_player + play_count) % 4;
                    Ok(Some((next_seat, ActionType::Play)))
                } else {
                    // First play of trick - need to determine leader
                    let leader = if current_trick_no == 0 {
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

    /// Execute an AI action with optional round context cache.
    ///
    /// If round_context is provided, uses cached data to build player view.
    /// Otherwise, loads all data from database (slower but always works).
    async fn execute_ai_action_with_cache(
        &self,
        txn: &DatabaseTransaction,
        game_id: i64,
        player_seat: i16,
        action_type: ActionType,
        ai: &dyn crate::ai::AiPlayer,
        round_context: Option<&crate::services::round_context::RoundContext>,
    ) -> Result<(), AppError> {
        use crate::domain::player_view::CurrentRoundInfo;

        // Build current round info - use cache if available
        let state = if let Some(context) = round_context {
            // Fast path: Use cached data
            let game = crate::adapters::games_sea::require_game(txn, game_id).await?;
            context
                .build_current_round_info(txn, player_seat, game.state, game.current_trick_no)
                .await?
        } else {
            // Fallback: Load everything from DB
            CurrentRoundInfo::load(txn, game_id, player_seat).await?
        };

        match action_type {
            ActionType::Bid => {
                let bid = ai.choose_bid(&state)?;
                // Use internal version to avoid recursion (loop handles processing)
                self.submit_bid_internal(txn, game_id, player_seat, bid)
                    .await?;
            }
            ActionType::Trump => {
                let trump_suit = ai.choose_trump(&state)?;
                // Convert Suit to rounds::Trump
                let trump = match trump_suit {
                    crate::domain::Suit::Clubs => rounds::Trump::Clubs,
                    crate::domain::Suit::Diamonds => rounds::Trump::Diamonds,
                    crate::domain::Suit::Hearts => rounds::Trump::Hearts,
                    crate::domain::Suit::Spades => rounds::Trump::Spades,
                };
                // Use internal version to avoid recursion (loop handles processing)
                self.set_trump_internal(txn, game_id, player_seat, trump)
                    .await?;
            }
            ActionType::Play => {
                let card = ai.choose_play(&state)?;
                // Use internal version to avoid recursion (loop handles processing)
                self.play_card_internal(txn, game_id, player_seat, card)
                    .await?;
            }
        }

        Ok(())
    }
}

/// Type of action needed from a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionType {
    Bid,
    Trump,
    Play,
}

impl Default for GameFlowService {
    fn default() -> Self {
        Self::new()
    }
}
