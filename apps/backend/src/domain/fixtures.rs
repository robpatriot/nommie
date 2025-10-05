use super::cards::Card;

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
    /// SAFETY: This function only accepts hardcoded valid card tokens that are
    /// known to parse successfully. The allow is necessary because the parser
    /// uses expect() for performance in fixture scenarios.
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
