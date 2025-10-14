//! Service layer and domain logic tests
//!
//! Tests for business logic, game flow, AI, domain invariants,
//! property tests, and snapshot serialization.
//!
//! Run all service tests:
//!   cargo test --test services_tests
//!
//! Run specific service tests:
//!   cargo test --test services_tests services::game_flow_happy_paths::

mod common;
mod support;

#[path = "suites/services"]
mod services {
    pub mod ai_memory;
    pub mod ai_memory_degradation;
    pub mod domain_bidding_props;
    pub mod domain_dealing_props;
    pub mod domain_error_mapping;
    pub mod domain_prop_tests_consistency;
    pub mod domain_prop_tests_legality;
    pub mod domain_prop_tests_trick_winner;
    pub mod domain_tricks_props;
    pub mod game;
    pub mod game_flow_happy_paths;
    pub mod game_flow_validations;
    pub mod game_round_progression_props;
    pub mod game_state_loading;
    pub mod games_snapshot;
    pub mod games_snapshot_caching;
    pub mod player_view;
    pub mod services_players;
    pub mod services_users;
    pub mod snapshot_phases;
}
