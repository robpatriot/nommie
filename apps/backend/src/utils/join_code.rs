//! Join code generation for games.
//!
//! This module provides utilities for generating unique join codes for games.
//! Join codes are 10-character strings using Crockford's Base32 alphabet.

use rand::distributions::Uniform;
use rand::prelude::*;
use rand::rngs::OsRng;

const CROCKFORD: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ"; // no I, L, O, U

/// Generate a unique join code for a game.
///
/// Creates a 10-character join code by randomly selecting characters from
/// Crockford's Base32 alphabet using the OS's cryptographically secure RNG.
///
/// # Returns
/// A 10-character string suitable for use as a game join code.
///
/// # Example
/// ```
/// use backend::utils::join_code::generate_join_code;
///
/// let code1 = generate_join_code();
/// let code2 = generate_join_code();
/// assert_ne!(code1, code2);
/// assert_eq!(code1.len(), 10);
/// ```
pub fn generate_join_code() -> String {
    let mut rng = OsRng;
    let dist = Uniform::from(0..CROCKFORD.len());

    let mut s = String::with_capacity(10);
    for _ in 0..10 {
        s.push(CROCKFORD[dist.sample(&mut rng)] as char);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_join_code_produces_different_results() {
        // Hash-based approach ensures uniqueness even for ULIDs generated in the same millisecond
        let code1 = generate_join_code();
        let code2 = generate_join_code();
        assert_ne!(code1, code2);
    }

    #[test]
    fn test_generate_join_code_has_correct_length() {
        let code = generate_join_code();
        assert_eq!(code.len(), 10);
    }
}
