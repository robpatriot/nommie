//! Domain layer: pure game logic types and helpers.

pub mod bidding;
pub mod cards_logic;
pub mod cards_parsing;
pub mod cards_serde;
pub mod cards_types;
pub mod dealing;
pub mod fixtures;
pub mod game_context;
pub mod player_view;
pub mod round_memory;
pub mod rules;
pub mod scoring;
pub mod seed_derivation;
pub mod snapshot;
pub mod state;
pub mod tricks;

#[cfg(test)]
mod domain_prop_helpers;
#[cfg(test)]
mod test_gens;
#[cfg(test)]
mod test_prelude;
#[cfg(test)]
mod tests_bidding;
#[cfg(test)]
mod tests_consecutive_zeros;
#[cfg(test)]
mod tests_conversions;
#[cfg(test)]
mod tests_domain_consistency;
#[cfg(test)]
mod tests_domain_dealing;
#[cfg(test)]
mod tests_integration;
#[cfg(test)]
mod tests_props_bidding;
#[cfg(test)]
mod tests_props_consistency;
#[cfg(test)]
mod tests_props_legality;
#[cfg(test)]
mod tests_props_trick_winner;
#[cfg(test)]
mod tests_props_tricks;
#[cfg(test)]
mod tests_scoring;
#[cfg(test)]
mod tests_snapshot_phases;
#[cfg(test)]
mod tests_tricks;

// Re-exports for ergonomics
pub use bidding::set_trump;
pub use cards_logic::{card_beats, hand_has_suit};
pub use cards_types::{Card, Rank, Suit, Trump};
pub use dealing::deal_hands;
pub use game_context::GameContext;
pub use round_memory::{PlayMemory, RankCategory, RoundMemory, TrickMemory};
pub use rules::{hand_size_for_round, valid_bid_range, PLAYERS};
pub use seed_derivation::{derive_dealing_seed, derive_memory_seed};
pub use snapshot::{GameSnapshot, PhaseSnapshot};
pub use state::{GameState, Phase, PlayerId, RoundState, Seat};
