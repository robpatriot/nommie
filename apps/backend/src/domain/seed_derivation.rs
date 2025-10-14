//! RNG seed derivation utilities for deterministic game behavior.
//!
//! Provides functions to derive unique-but-deterministic seeds for different
//! game contexts (dealing, memory degradation, etc.) from a base game seed.

/// Derive a seed for AI memory degradation.
///
/// Creates a unique seed for each AI's memory of a specific round, ensuring:
/// - Same game + round + player = same memory degradation
/// - Different players forget different cards (even with same memory_level)
/// - Memory is stable throughout a round (idempotent)
///
/// # Arguments
///
/// * `game_seed` - Base RNG seed from the game (games.rng_seed)
/// * `round_no` - Round number (0-25)
/// * `player_seat` - Player seat (0-3)
///
/// # Returns
///
/// Derived seed that is unique per (game, round, player) combination.
pub fn derive_memory_seed(game_seed: i64, round_no: i16, player_seat: i16) -> u64 {
    // Cast i64 to u64 for RNG (sign doesn't matter for seed)
    let base = game_seed as u64;

    // Simple arithmetic derivation for deterministic but unique seeds
    // Uses different multipliers to avoid collisions between contexts
    base.wrapping_add((round_no as u64).wrapping_mul(10000))
        .wrapping_add((player_seat as u64).wrapping_mul(100))
        .wrapping_add(1) // Offset to distinguish from dealing seed
}

/// Derive a seed for dealing cards in a round.
///
/// Creates a unique seed for dealing each round's hands.
///
/// # Arguments
///
/// * `game_seed` - Base RNG seed from the game (games.rng_seed)
/// * `round_no` - Round number (0-25)
///
/// # Returns
///
/// Derived seed that is unique per (game, round) combination.
pub fn derive_dealing_seed(game_seed: i64, round_no: i16) -> u64 {
    // Cast i64 to u64 for RNG (sign doesn't matter for seed)
    let base = game_seed as u64;

    // Different multiplier from memory to ensure separation
    base.wrapping_add((round_no as u64).wrapping_mul(1000000))
        .wrapping_add(2) // Offset to distinguish from memory seed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_seed_uniqueness() {
        let base = 12345i64;

        // Same inputs produce same output
        let seed1 = derive_memory_seed(base, 5, 2);
        let seed2 = derive_memory_seed(base, 5, 2);
        assert_eq!(seed1, seed2, "Same inputs should produce same seed");

        // Different rounds produce different seeds
        let seed_r1 = derive_memory_seed(base, 1, 0);
        let seed_r2 = derive_memory_seed(base, 2, 0);
        assert_ne!(
            seed_r1, seed_r2,
            "Different rounds should produce different seeds"
        );

        // Different players produce different seeds
        let seed_p0 = derive_memory_seed(base, 1, 0);
        let seed_p1 = derive_memory_seed(base, 1, 1);
        assert_ne!(
            seed_p0, seed_p1,
            "Different players should produce different seeds"
        );

        // Different games produce different seeds
        let seed_g1 = derive_memory_seed(12345, 1, 0);
        let seed_g2 = derive_memory_seed(67890, 1, 0);
        assert_ne!(
            seed_g1, seed_g2,
            "Different games should produce different seeds"
        );
    }

    #[test]
    fn test_dealing_seed_uniqueness() {
        let base = 12345i64;

        // Same inputs produce same output
        let seed1 = derive_dealing_seed(base, 5);
        let seed2 = derive_dealing_seed(base, 5);
        assert_eq!(seed1, seed2, "Same inputs should produce same seed");

        // Different rounds produce different seeds
        let seed_r1 = derive_dealing_seed(base, 1);
        let seed_r2 = derive_dealing_seed(base, 2);
        assert_ne!(
            seed_r1, seed_r2,
            "Different rounds should produce different seeds"
        );
    }

    #[test]
    fn test_memory_vs_dealing_separation() {
        let base = 12345i64;
        let round = 5i16;

        // Memory and dealing seeds should be different even for same round
        let memory_seed = derive_memory_seed(base, round, 0);
        let dealing_seed = derive_dealing_seed(base, round);
        assert_ne!(
            memory_seed, dealing_seed,
            "Memory and dealing seeds should be different"
        );
    }

    #[test]
    fn test_wrapping_behavior() {
        // Test with values that would overflow
        let large_seed = i64::MAX - 1000;
        let seed1 = derive_memory_seed(large_seed, 25, 3);
        let seed2 = derive_memory_seed(large_seed, 25, 3);
        assert_eq!(seed1, seed2, "Wrapping should be deterministic");
    }
}
