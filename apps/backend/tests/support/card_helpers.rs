//! Test helpers for card parsing and fixtures

use backend::domain::Card;
use backend::errors::domain::DomainError;

/// Non-panicking helper to parse card tokens (e.g., "AS", "2C") into Card instances.
/// Returns Result<Vec<Card>, DomainError> if any token is invalid.
pub fn try_parse_cards<I, S>(tokens: I) -> Result<Vec<Card>, DomainError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    tokens
        .into_iter()
        .map(|s| s.as_ref().parse::<Card>())
        .collect()
}

/// Centralized helper for parsing hardcoded card tokens in fixtures and demo data.
///
/// This module contains a single allow for parsing hardcoded card tokens that are
/// known to be valid at compile time. Used only for demo data and fixtures.
pub struct CardFixtures;

impl CardFixtures {
    /// Parse hardcoded card tokens into Card instances.
    ///
    /// This function is intended only for use with hardcoded valid card tokens
    /// in fixtures, demo data, and test scenarios. The tokens are validated
    /// at compile time and cannot fail at runtime.
    ///
    /// # Arguments
    /// * `tokens` - Slice of hardcoded card token strings (e.g., ["AS", "2C", "TH"])
    ///
    /// # Returns
    /// Vector of parsed Card instances
    ///
    /// # Safety
    /// This function only accepts hardcoded valid card tokens that are
    /// known to parse successfully. The `#[allow(clippy::expect_used)]` is
    /// necessary because this function uses `.expect()` for performance in
    /// fixture scenarios where parse failures should never occur.
    pub fn parse_hardcoded(tokens: &[&str]) -> Vec<Card> {
        tokens
            .iter()
            .map(|s| {
                #[allow(clippy::expect_used)]
                s.parse::<Card>().expect("hardcoded valid card token")
            })
            .collect()
    }
}
