//! Domain layer: pure game logic types and helpers.

pub mod bidding;
pub mod cards_logic;
pub mod cards_parsing;
pub mod cards_serde;
pub mod cards_types;
pub mod dealing;
pub mod fixtures;
pub mod rules;
pub mod scoring;
pub mod snapshot;
pub mod state;
pub mod tricks;

#[cfg(test)]
mod tests_bidding;
#[cfg(test)]
mod tests_conversions;
#[cfg(test)]
mod tests_integration;
#[cfg(test)]
mod tests_scoring;
#[cfg(test)]
mod tests_tricks;

// Re-exports for ergonomics
pub use bidding::set_trump;
pub use cards_logic::{card_beats, hand_has_suit};
pub use cards_types::{Card, Rank, Suit, Trump};
pub use dealing::deal_hands;
pub use rules::{hand_size_for_round, valid_bid_range, PLAYERS};
pub use snapshot::{GameSnapshot, PhaseSnapshot};
pub use state::{GameState, Phase, PlayerId, RoundState, Seat};
