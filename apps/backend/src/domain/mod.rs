//! Domain layer: pure game logic types and helpers.

pub mod bidding;
pub mod cards;
pub mod errors;
pub mod fixtures;
pub mod rules;
pub mod scoring;
pub mod snapshot;
pub mod state;
pub mod tests;
pub mod tricks;

// Re-exports for ergonomics
pub use bidding::set_trump;
pub use cards::{card_beats, hand_has_suit, Card, Rank, Suit, Trump};
pub use errors::DomainError;
pub use rules::{hand_size_for_round, valid_bid_range, PLAYERS};
pub use snapshot::{GameSnapshot, PhaseSnapshot};
pub use state::{GameState, Phase, PlayerId, RoundState, Seat};
