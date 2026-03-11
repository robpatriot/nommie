// Authentication and authorization tests
//
// Tests for authentication flows, JWT validation, auth extractors, and authz.
//
// Run all auth tests:
//   cargo test --test auth_tests
//
// Run specific auth tests:
//   cargo test --test auth_tests auth::login::

mod common;
mod support;

#[path = "suites/auth/mod.rs"]
mod auth;

#[path = "suites/authz/mod.rs"]
mod authz;
