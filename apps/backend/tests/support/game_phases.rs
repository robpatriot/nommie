// Game phase setup helpers for integration tests
//
// This module provides helpers for setting up games in specific phases
// (Bidding, TrumpSelection, TrickPlay) to reduce boilerplate in tests.

use backend::error::AppError;
use backend::repos::rounds::Trump;
use backend::services::game_flow::GameFlowService;
use sea_orm::DatabaseTransaction;

use super::game_setup::setup_game_with_players;

/// Result of a phase setup operation with additional context
pub struct PhaseSetup {
    pub game_id: i64,
    pub user_ids: Vec<i64>,
    pub round_id: i64,
    pub dealer_pos: u8,
}

/// Set up a game in the Bidding phase (dealt, ready to bid).
///
/// Creates a game with 4 ready players and deals the first round.
/// Game will be in Bidding state after this call.
///
/// # Arguments
/// * `txn` - Database transaction
/// * `test_name` - Test name for unique seed generation
///
/// # Returns
/// PhaseSetup with game_id, user_ids, round_id, and dealer position
///
/// # Example
/// ```
/// let setup = setup_game_in_bidding_phase(txn, "my_bidding_test").await?;
/// // Now submit bids...
/// service.submit_bid(txn, setup.game_id, 1, 5, None).await?;
/// ```
pub async fn setup_game_in_bidding_phase(
    txn: &DatabaseTransaction,
    test_name: &str,
) -> Result<PhaseSetup, AppError> {
    let game_setup = setup_game_with_players(txn, test_name).await?;
    let service = GameFlowService;

    // Deal the round
    service.deal_round(txn, game_setup.game_id).await?;

    // Get round info
    let game = backend::adapters::games_sea::require_game(txn, game_setup.game_id).await?;
    let round_no: u8 = game
        .current_round
        .and_then(|value| value.try_into().ok())
        .expect("Game should have current round");
    let dealer_pos = game.dealer_pos().expect("Game should have dealer position");

    let round = backend::repos::rounds::find_by_game_and_round(txn, game_setup.game_id, round_no)
        .await?
        .expect("Round should exist");

    Ok(PhaseSetup {
        game_id: game_setup.game_id,
        user_ids: game_setup.user_ids,
        round_id: round.id,
        dealer_pos,
    })
}

/// Set up a game in TrumpSelection phase (dealt + all bids submitted).
///
/// Creates a game, deals, and submits all 4 bids in correct dealer order.
/// Game will be in TrumpSelection state after this call.
///
/// # Arguments
/// * `txn` - Database transaction
/// * `test_name` - Test name for unique seed generation
/// * `bids` - Array of 4 bid values (indexed by seat, not bid order)
///
/// # Returns
/// PhaseSetup with game_id, user_ids, round_id, and dealer position
///
/// # Example
/// ```
/// let setup = setup_game_in_trump_selection_phase(txn, "my_trump_test", [3, 4, 2, 5]).await?;
/// // Now set trump...
/// service.set_trump(txn, setup.game_id, winning_bidder, Trump::Hearts, None).await?;
/// ```
pub async fn setup_game_in_trump_selection_phase(
    txn: &DatabaseTransaction,
    test_name: &str,
    bids: [u8; 4],
) -> Result<PhaseSetup, AppError> {
    let phase_setup = setup_game_in_bidding_phase(txn, test_name).await?;
    let service = GameFlowService;

    // Submit all bids in dealer order
    // Bidding starts at (dealer_pos + 1) % 4
    let first_bidder = ((phase_setup.dealer_pos + 1) % 4) as usize;
    for i in 0..4 {
        let seat = (first_bidder + i) % 4;
        service
            .submit_bid(txn, phase_setup.game_id, seat as u8, bids[seat], None)
            .await?;
    }

    Ok(phase_setup)
}

/// Set up a game in TrickPlay phase (dealt + bids + trump selected).
///
/// Creates a game, deals, submits all bids, and sets trump.
/// Game will be in TrickPlay state after this call.
///
/// # Arguments
/// * `txn` - Database transaction
/// * `test_name` - Test name for unique seed generation
/// * `bids` - Array of 4 bid values (indexed by seat)
/// * `trump` - Trump suit to set
///
/// # Returns
/// PhaseSetup with game_id, user_ids, round_id, and dealer position
///
/// # Example
/// ```
/// let setup = setup_game_in_trick_play_phase(txn, "my_trick_test", [3, 4, 2, 5], Trump::Hearts).await?;
/// // Now play cards...
/// service.play_card(txn, setup.game_id, 0, card, None).await?;
/// ```
pub async fn setup_game_in_trick_play_phase(
    txn: &DatabaseTransaction,
    test_name: &str,
    bids: [u8; 4],
    trump: Trump,
) -> Result<PhaseSetup, AppError> {
    let phase_setup = setup_game_in_trump_selection_phase(txn, test_name, bids).await?;
    let service = GameFlowService;

    // Determine winning bidder (highest bid, ties go to earliest bidder)
    let winning_bidder = find_winning_bidder(&bids, phase_setup.dealer_pos);

    // Set trump
    service
        .set_trump(txn, phase_setup.game_id, winning_bidder, trump, None)
        .await?;

    Ok(phase_setup)
}

/// Find the winning bidder given bids and dealer position.
///
/// Winner is the player with the highest bid. In case of tie, the earlier bidder wins.
/// Bidding order starts at (dealer_pos + 1) % 4.
pub fn find_winning_bidder(bids: &[u8; 4], dealer_pos: u8) -> u8 {
    let first_bidder = ((dealer_pos + 1) % 4) as usize;
    let mut max_bid = 0;
    let mut winner = first_bidder;

    for i in 0..4 {
        let seat = (first_bidder + i) % 4;
        if bids[seat] > max_bid {
            max_bid = bids[seat];
            winner = seat;
        }
    }

    winner as u8
}

/// Set up a game at a specific round number (between rounds).
///
/// Creates a game in BetweenRounds state with:
/// - 4 ready players
/// - All previous rounds completed with simple scoring
/// - Ready to deal the next round
///
/// # Arguments
/// * `txn` - Database transaction
/// * `test_name` - Test name for unique seed generation
/// * `completed_rounds` - Number of rounds already completed (1-26)
///
/// # Returns
/// PhaseSetup with game_id, user_ids, round_id (of last completed round), and dealer position
///
/// # Example
/// ```
/// // Set up game with 25 rounds completed, ready to deal round 26
/// let setup = setup_game_at_round(txn, "my_multi_round_test", 25).await?;
/// // Now deal the next round...
/// service.deal_round(txn, setup.game_id).await?;
/// ```
pub async fn setup_game_at_round(
    txn: &DatabaseTransaction,
    test_name: &str,
    completed_rounds: i32,
) -> Result<PhaseSetup, AppError> {
    use backend::entities::games::{self, GameState, GameVisibility};
    use backend::repos::{bids, memberships, rounds, scores};
    use sea_orm::{ActiveModelTrait, NotSet, Set};
    use time::OffsetDateTime;

    if !(0..=26).contains(&completed_rounds) {
        return Err(AppError::from(
            backend::errors::domain::DomainError::validation(
                backend::errors::domain::ValidationKind::Other("INVALID_ROUND".into()),
                format!("completed_rounds must be 0-26, got {}", completed_rounds),
            ),
        ));
    }

    // Create 4 test users
    let user_ids = vec![
        super::factory::create_test_user(
            txn,
            &super::test_utils::test_user_sub(&format!("{}_p1", test_name)),
            Some("Player 1"),
        )
        .await?,
        super::factory::create_test_user(
            txn,
            &super::test_utils::test_user_sub(&format!("{}_p2", test_name)),
            Some("Player 2"),
        )
        .await?,
        super::factory::create_test_user(
            txn,
            &super::test_utils::test_user_sub(&format!("{}_p3", test_name)),
            Some("Player 3"),
        )
        .await?,
        super::factory::create_test_user(
            txn,
            &super::test_utils::test_user_sub(&format!("{}_p4", test_name)),
            Some("Player 4"),
        )
        .await?,
    ];

    // Create game in BetweenRounds state (or Lobby if 0 rounds completed)
    let now = OffsetDateTime::now_utc();
    let game_state = if completed_rounds == 0 {
        GameState::Lobby
    } else {
        GameState::BetweenRounds
    };

    let game = games::ActiveModel {
        id: NotSet,
        created_by: Set(Some(user_ids[0])),
        visibility: Set(GameVisibility::Public),
        state: Set(game_state),
        created_at: Set(now),
        updated_at: Set(now),
        started_at: Set(if completed_rounds > 0 {
            Some(now)
        } else {
            None
        }),
        ended_at: Set(None),
        name: Set(Some(format!("Test Game at Round {}", completed_rounds))),
        join_code: Set(Some(format!(
            "R{}{}",
            completed_rounds,
            rand::random::<u32>() % 100000
        ))),
        rules_version: Set("1.0".to_string()),
        rng_seed: Set(Some(super::test_utils::test_seed(test_name))),
        current_round: Set(if completed_rounds > 0 {
            Some(completed_rounds as i16)
        } else {
            None
        }),
        starting_dealer_pos: Set(if completed_rounds > 0 {
            Some(0i16)
        } else {
            None
        }),
        current_trick_no: Set(0i16),
        current_round_id: Set(None), // No active round
        lock_version: Set(0),
    };
    let game_id = game.insert(txn).await?.id;

    // Add players as memberships
    for (i, user_id) in user_ids.iter().enumerate() {
        memberships::create_membership(
            txn,
            game_id,
            *user_id,
            i as u8,
            true, // All ready
            memberships::GameRole::Player,
        )
        .await?;
    }

    let mut last_round_id = 0;

    // Create only the last completed round with accumulated scores
    // This is much faster than creating all previous rounds
    if completed_rounds > 0 {
        let round_no = completed_rounds;
        let hand_size =
            backend::domain::rules::hand_size_for_round(round_no as u8).ok_or_else(|| {
                AppError::from(backend::errors::domain::DomainError::validation(
                    backend::errors::domain::ValidationKind::InvalidHandSize,
                    format!("Invalid round number: {}", round_no),
                ))
            })?;

        let dealer_pos = ((round_no - 1) % 4) as u8;

        let round =
            rounds::create_round(txn, game_id, round_no as u8, hand_size, dealer_pos).await?;

        last_round_id = round.id;

        // Create simple bids and scores for each player
        // Each player gets 4 points per round (simplified for test setup)
        // total_score_after represents accumulated score from all completed rounds
        for seat in 0..4 {
            // Create bid records (bid_order = seat for simplicity)
            bids::create_bid(txn, round.id, seat, 1, seat).await?;

            // Create score records with accumulated totals
            scores::create_score(
                txn,
                scores::ScoreData {
                    round_id: round.id,
                    player_seat: seat,
                    bid_value: 1,
                    tricks_won: 1,
                    bid_met: false,
                    base_score: 1,
                    bonus: 0,
                    round_score: 4,
                    total_score_after: (4 * round_no) as i16,
                },
            )
            .await?;
        }

        // Mark round as completed
        rounds::complete_round(txn, round.id).await?;
    }

    Ok(PhaseSetup {
        game_id,
        user_ids,
        round_id: last_round_id,
        dealer_pos: if completed_rounds > 0 {
            ((completed_rounds - 1) % 4) as u8
        } else {
            0u8
        },
    })
}
