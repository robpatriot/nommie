//! Trick creation helpers for integration tests
//!
//! This module provides helpers for creating tricks with known winners,
//! reducing boilerplate in tests that need to simulate complete rounds.

use backend::error::AppError;
use backend::repos::tricks;
use sea_orm::DatabaseTransaction;

/// Create multiple tricks with specified winners.
///
/// This is useful for simulating a completed round where you know which player
/// won each trick. The tricks are numbered sequentially starting from the first
/// trick number provided.
///
/// # Arguments
/// * `txn` - Database transaction
/// * `round_id` - ID of the round these tricks belong to
/// * `winners` - Slice of seat numbers (0-3) indicating who won each trick
/// * `starting_trick_no` - The trick number to start from (usually 0)
///
/// # Returns
/// Vector of created trick IDs
///
/// # Example
/// ```
/// // Create 13 tricks: P0 wins 3, P1 wins 3, P2 wins 4, P3 wins 3
/// let trick_ids = create_tricks_with_winners(
///     txn,
///     round_id,
///     &[0, 0, 0, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3],
///     0
/// ).await?;
/// ```
pub async fn create_tricks_with_winners(
    txn: &DatabaseTransaction,
    round_id: i64,
    winners: &[i16],
    starting_trick_no: i16,
) -> Result<Vec<i64>, AppError> {
    let mut trick_ids = Vec::with_capacity(winners.len());

    for (idx, &winner_seat) in winners.iter().enumerate() {
        let trick_no = starting_trick_no + idx as i16;

        // Use a default suit based on winner to have some variety
        let suit = match winner_seat % 4 {
            0 => tricks::Suit::Hearts,
            1 => tricks::Suit::Spades,
            2 => tricks::Suit::Clubs,
            _ => tricks::Suit::Diamonds,
        };

        let trick = tricks::create_trick(txn, round_id, trick_no, suit, winner_seat).await?;
        trick_ids.push(trick.id);
    }

    Ok(trick_ids)
}

/// Create tricks with winners grouped by player.
///
/// This is a convenience wrapper for common patterns where you know how many
/// tricks each player won. It creates the tricks in player order (all of P0's wins,
/// then all of P1's wins, etc.).
///
/// # Arguments
/// * `txn` - Database transaction
/// * `round_id` - ID of the round these tricks belong to
/// * `tricks_per_player` - Array of 4 values indicating how many tricks each player (0-3) won
///
/// # Returns
/// Vector of created trick IDs
///
/// # Example
/// ```
/// // P0 wins 5, P1 wins 4, P2 wins 3, P3 wins 1 = 13 total tricks
/// let trick_ids = create_tricks_by_winner_counts(
///     txn,
///     round_id,
///     [5, 4, 3, 1]
/// ).await?;
/// ```
pub async fn create_tricks_by_winner_counts(
    txn: &DatabaseTransaction,
    round_id: i64,
    tricks_per_player: [u8; 4],
) -> Result<Vec<i64>, AppError> {
    let mut winners = Vec::new();

    for (seat, &count) in tricks_per_player.iter().enumerate() {
        for _ in 0..count {
            winners.push(seat as i16);
        }
    }

    create_tricks_with_winners(txn, round_id, &winners, 0).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_winner_counts_to_winners_list() {
        let tricks_per_player = [3, 4, 2, 4];
        let mut winners = Vec::new();

        for (seat, &count) in tricks_per_player.iter().enumerate() {
            for _ in 0..count {
                winners.push(seat as i16);
            }
        }

        assert_eq!(winners.len(), 13);
        assert_eq!(winners[0..3], [0, 0, 0]); // P0 wins first 3
        assert_eq!(winners[3..7], [1, 1, 1, 1]); // P1 wins next 4
        assert_eq!(winners[7..9], [2, 2]); // P2 wins next 2
        assert_eq!(winners[9..13], [3, 3, 3, 3]); // P3 wins last 4
    }
}
