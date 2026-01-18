use crate::errors::domain::{DomainError, ValidationKind};

/// Validate that a seed is exactly 32 bytes and return it as a fixed-size array.
///
/// This is the centralized validation point for converting `Vec<u8>` or slices
/// to the `[u8; 32]` required by cryptographic functions.
///
/// # Errors
/// Fail with `DomainError` if length is not exactly 32.
pub fn require_seed_32(seed: &[u8]) -> Result<[u8; 32], DomainError> {
    seed.try_into().map_err(|_| {
        DomainError::validation(
            ValidationKind::Other("INVALID_SEED_LENGTH".into()),
            format!("Seed must be exactly 32 bytes, got {}", seed.len()),
        )
    })
}

/// Derive a seed for AI memory degradation.
///
/// Creates a unique seed for each AI's memory of a specific round, ensuring:
/// - Same game + round + player = same memory degradation
/// - Different players forget different cards (even with same memory_level)
/// - Memory is stable throughout a round (idempotent)
///
/// # Arguments
///
/// * `game_seed` - Base 32-byte RNG seed from the game (games.rng_seed)
/// * `round_no` - Round number (0-25)
/// * `player_seat` - Player seat (0-3)
///
/// # Returns
///
/// Derived u64 seed that is unique per (game, round, player) combination.
pub fn derive_memory_seed(
    game_seed: &[u8; 32],
    round_no: u8,
    player_seat: u8,
) -> Result<u64, DomainError> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"nommie/memory/v1");
    hasher.update(game_seed);
    hasher.update(&[round_no]);
    hasher.update(&[player_seat]);
    let hash = hasher.finalize();
    // Take first 8 bytes as u64
    let bytes: [u8; 8] = hash.as_bytes()[..8].try_into().map_err(|_| {
        DomainError::infra(
            crate::errors::domain::InfraErrorKind::DataCorruption,
            "failed to extract bytes for memory seed",
        )
    })?;
    Ok(u64::from_le_bytes(bytes))
}

/// Derive a seed for dealing cards in a round.
///
/// Creates a unique 32-byte seed for dealing each round's hands using BLAKE3.
///
/// # Arguments
///
/// * `game_seed` - Base 32-byte RNG seed from the game (games.rng_seed)
/// * `round_no` - Round number (0-25)
///
/// # Returns
///
/// Derived 32-byte seed that is unique per (game, round) combination.
pub fn derive_dealing_seed(game_seed: &[u8; 32], round_no: u8) -> Result<[u8; 32], DomainError> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"nommie/deal/v1");
    hasher.update(game_seed);
    hasher.update(&[round_no]);
    Ok(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_seed_uniqueness() {
        let base: [u8; 32] = [0x42; 32];

        // Same inputs produce same output
        let seed1 = derive_memory_seed(&base, 5, 2).unwrap();
        let seed2 = derive_memory_seed(&base, 5, 2).unwrap();
        assert_eq!(seed1, seed2, "Same inputs should produce same seed");

        // Different rounds produce different seeds
        let seed_r1 = derive_memory_seed(&base, 1, 0);
        let seed_r2 = derive_memory_seed(&base, 2, 0);
        assert_ne!(
            seed_r1, seed_r2,
            "Different rounds should produce different seeds"
        );

        // Different players produce different seeds
        let seed_p0 = derive_memory_seed(&base, 1, 0);
        let seed_p1 = derive_memory_seed(&base, 1, 1);
        assert_ne!(
            seed_p0, seed_p1,
            "Different players should produce different seeds"
        );

        // Different games produce different seeds
        let base1: [u8; 32] = [0x12; 32];
        let base2: [u8; 32] = [0x67; 32];
        let seed_g1 = derive_memory_seed(&base1, 1, 0);
        let seed_g2 = derive_memory_seed(&base2, 1, 0);
        assert_ne!(
            seed_g1, seed_g2,
            "Different games should produce different seeds"
        );
    }

    #[test]
    fn test_dealing_seed_uniqueness() {
        let base: [u8; 32] = [0x42; 32];

        // Same inputs produce same output
        let seed1 = derive_dealing_seed(&base, 5);
        let seed2 = derive_dealing_seed(&base, 5);
        assert_eq!(seed1, seed2, "Same inputs should produce same seed");

        // Different rounds produce different seeds
        let seed_r1 = derive_dealing_seed(&base, 1);
        let seed_r2 = derive_dealing_seed(&base, 2);
        assert_ne!(
            seed_r1, seed_r2,
            "Different rounds should produce different seeds"
        );
    }

    #[test]
    fn test_memory_vs_dealing_separation() {
        let base: [u8; 32] = [0x42; 32];
        let round = 5u8;

        // Memory and dealing seeds should be different even for same round
        let memory_seed = derive_memory_seed(&base, round, 0).unwrap();
        let dealing_seed = derive_dealing_seed(&base, round).unwrap();
        // Convert dealing_seed to u64 for comparison
        let dealing_u64 = u64::from_le_bytes(dealing_seed[..8].try_into().unwrap());
        assert_ne!(
            memory_seed, dealing_u64,
            "Memory and dealing seeds should be different"
        );
    }

    #[test]
    fn test_determinism() {
        // Test that the output is exactly the same across runs
        let seed: [u8; 32] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x20,
        ];
        let dealing1 = derive_dealing_seed(&seed, 1).unwrap();
        let dealing2 = derive_dealing_seed(&seed, 1).unwrap();
        assert_eq!(dealing1, dealing2, "Dealing seeds must be deterministic");
    }

    #[test]
    fn test_require_seed_32() {
        let valid: Vec<u8> = vec![0u8; 32];
        let result = require_seed_32(&valid);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), [0u8; 32]);

        let too_short: Vec<u8> = vec![0u8; 31];
        let result = require_seed_32(&too_short);
        assert!(result.is_err());

        let too_long: Vec<u8> = vec![0u8; 33];
        let result = require_seed_32(&too_long);
        assert!(result.is_err());
    }
}
