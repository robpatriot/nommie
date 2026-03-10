// Test support is shared across multiple test binaries (adapters, auth, routes, services, etc.).
// Each binary uses a subset of helpers; the rest appear "dead" when that binary is analyzed.
#![allow(dead_code, unused_imports)]

pub mod ai_memory_helpers;
pub mod app_builder;
pub mod auth;
pub mod card_helpers;
pub mod db_games;
pub mod db_memberships;
pub mod factory;
pub mod game_phases;
pub mod game_setup;
pub mod game_state;
pub mod games_sea_helpers;
pub mod snapshot_helpers;
pub mod state_helpers;
pub mod test_middleware;
pub mod test_state;
pub mod test_utils;
pub mod trick_helpers;
pub mod txn_helpers;
pub mod websocket;
pub mod websocket_client;

pub use test_state::{build_test_state, test_state_builder};
pub use test_utils::test_seed;
