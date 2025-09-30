//! Domain layer: pure game logic types and helpers.

pub mod cards;
pub mod rules;
pub mod errors;
pub mod state;
pub mod bidding;
pub mod tricks;
pub mod scoring;

// Re-exports for ergonomics
pub use cards::{Suit, Rank, Card, card_beats, hand_has_suit};
pub use rules::{PLAYERS, hand_size_for_round, valid_bid_range};
pub use errors::DomainError;
pub use state::{PlayerId, Phase, GameState, RoundState};
pub use bidding::set_trump;


