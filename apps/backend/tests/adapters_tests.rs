// Adapter tests
//
// Tests for SeaORM adapters (repositories) covering CRUD operations,
// constraints, and database-level invariants.
//
// Run all adapter tests:
//   cargo test --test adapters_tests
//
// Run specific adapter tests:
//   cargo test --test adapters_tests adapters::games_sea::

mod common;
mod support;

#[path = "suites/adapters/mod.rs"]
mod adapters;
