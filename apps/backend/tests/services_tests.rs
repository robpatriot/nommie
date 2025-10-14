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

#[path = "suites/services/mod.rs"]
mod services;
