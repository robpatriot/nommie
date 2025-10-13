//! Game phase setup helpers for integration tests
//!
//! This module provides helpers for setting up games in specific phases
//! (Bidding, TrumpSelection, TrickPlay) to reduce boilerplate in tests.

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
    pub dealer_pos: i32,
}

/// Set up a game in the Bidding phase (dealt, ready to bid).
///
/// Creates a game with 4 ready players and deals the first round.
/// Game will be in Bidding state after this call.
///
/// # Arguments
/// * `txn` - Database transaction
/// * `rng_seed` - Seed for deterministic RNG
///
/// # Returns
/// PhaseSetup with game_id, user_ids, round_id, and dealer position
///
/// # Example
/// ```
/// let setup = setup_game_in_bidding_phase(txn, 12345).await?;
/// // Now submit bids...
/// service.submit_bid(txn, setup.game_id, 1, 5).await?;
/// ```
pub async fn setup_game_in_bidding_phase(
    txn: &DatabaseTransaction,
    rng_seed: i64,
) -> Result<PhaseSetup, AppError> {
    let game_setup = setup_game_with_players(txn, rng_seed).await?;
    let service = GameFlowService::new();

    // Deal the round
    service.deal_round(txn, game_setup.game_id).await?;

    // Get round info
    let game = backend::adapters::games_sea::require_game(txn, game_setup.game_id).await?;
    let round_no = game.current_round.expect("Game should have current round");
    let dealer_pos = game.dealer_pos().expect("Game should have dealer position");

    let round = backend::repos::rounds::find_by_game_and_round(txn, game_setup.game_id, round_no)
        .await?
        .expect("Round should exist");

    Ok(PhaseSetup {
        game_id: game_setup.game_id,
        user_ids: game_setup.user_ids,
        round_id: round.id,
        dealer_pos: dealer_pos as i32,
    })
}

/// Set up a game in TrumpSelection phase (dealt + all bids submitted).
///
/// Creates a game, deals, and submits all 4 bids in correct dealer order.
/// Game will be in TrumpSelection state after this call.
///
/// # Arguments
/// * `txn` - Database transaction
/// * `rng_seed` - Seed for deterministic RNG
/// * `bids` - Array of 4 bid values (indexed by seat, not bid order)
///
/// # Returns
/// PhaseSetup with game_id, user_ids, round_id, and dealer position
///
/// # Example
/// ```
/// let setup = setup_game_in_trump_selection_phase(txn, 12345, [3, 4, 2, 5]).await?;
/// // Now set trump...
/// service.set_trump(txn, setup.game_id, winning_bidder, Trump::Hearts).await?;
/// ```
pub async fn setup_game_in_trump_selection_phase(
    txn: &DatabaseTransaction,
    rng_seed: i64,
    bids: [u8; 4],
) -> Result<PhaseSetup, AppError> {
    let phase_setup = setup_game_in_bidding_phase(txn, rng_seed).await?;
    let service = GameFlowService::new();

    // Submit all bids in dealer order
    // Bidding starts at (dealer_pos + 1) % 4
    let first_bidder = ((phase_setup.dealer_pos + 1) % 4) as usize;
    for i in 0..4 {
        let seat = (first_bidder + i) % 4;
        service
            .submit_bid(txn, phase_setup.game_id, seat as i16, bids[seat])
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
/// * `rng_seed` - Seed for deterministic RNG
/// * `bids` - Array of 4 bid values (indexed by seat)
/// * `trump` - Trump suit to set
///
/// # Returns
/// PhaseSetup with game_id, user_ids, round_id, and dealer position
///
/// # Example
/// ```
/// let setup = setup_game_in_trick_play_phase(txn, 12345, [3, 4, 2, 5], Trump::Hearts).await?;
/// // Now play cards...
/// service.play_card(txn, setup.game_id, 0, card).await?;
/// ```
pub async fn setup_game_in_trick_play_phase(
    txn: &DatabaseTransaction,
    rng_seed: i64,
    bids: [u8; 4],
    trump: Trump,
) -> Result<PhaseSetup, AppError> {
    let phase_setup = setup_game_in_trump_selection_phase(txn, rng_seed, bids).await?;
    let service = GameFlowService::new();

    // Determine winning bidder (highest bid, ties go to earliest bidder)
    let winning_bidder = find_winning_bidder(&bids, phase_setup.dealer_pos);

    // Set trump
    service
        .set_trump(txn, phase_setup.game_id, winning_bidder as i16, trump)
        .await?;

    Ok(phase_setup)
}

/// Find the winning bidder given bids and dealer position.
///
/// Winner is the player with the highest bid. In case of tie, the earlier bidder wins.
/// Bidding order starts at (dealer_pos + 1) % 4.
fn find_winning_bidder(bids: &[u8; 4], dealer_pos: i32) -> i32 {
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

    winner as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_winning_bidder_simple() {
        let bids = [3, 5, 2, 4];
        let dealer_pos = 0;
        assert_eq!(find_winning_bidder(&bids, dealer_pos), 1); // Seat 1 has highest bid
    }

    #[test]
    fn test_find_winning_bidder_tie_early_wins() {
        let bids = [3, 4, 4, 2]; // Seats 1 and 2 both bid 4
        let dealer_pos = 0;
        // Bidding order: 1, 2, 3, 0
        // Seat 1 bids first with 4, seat 2 bids second with 4
        // Seat 1 should win (earlier bidder)
        assert_eq!(find_winning_bidder(&bids, dealer_pos), 1);
    }

    #[test]
    fn test_find_winning_bidder_dealer_not_first() {
        let bids = [5, 3, 2, 4];
        let dealer_pos = 1;
        // Bidding order: 2, 3, 0, 1
        // Seat 0 has highest bid (5)
        assert_eq!(find_winning_bidder(&bids, dealer_pos), 0);
    }
}
