//! Adapter tests
//!
//! Tests for SeaORM adapters (repositories) covering CRUD operations,
//! constraints, and database-level invariants.
//!
//! Run all adapter tests:
//!   cargo test --test adapters_tests
//!
//! Run specific adapter tests:
//!   cargo test --test adapters_tests adapters::games_sea::

mod common;
mod support;

#[path = "suites/adapters"]
mod adapters {
    pub mod bids_sea;
    pub mod games_sea;
    pub mod hands_sea;
    pub mod memberships_sea;
    pub mod optimistic_lock_repo_tests;
    pub mod players_sea;
    pub mod plays_sea;
    pub mod rounds_sea;
    pub mod scores_sea;
    pub mod tricks_sea;
    pub mod users_sea;
}
