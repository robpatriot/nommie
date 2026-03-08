//! Test helpers for card parsing and fixtures

use backend::domain::Card;
use backend::errors::domain::DomainError;

/// Parse card tokens (e.g., "AS", "2C") into Card instances.
/// Returns `Result<Vec<Card>, DomainError>` if any token is invalid.
///
/// Prefer this over panicking helpers. In fixtures where tokens are known valid,
/// call `try_parse_cards(tokens).expect("fixture data must be valid")` at the call site.
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
