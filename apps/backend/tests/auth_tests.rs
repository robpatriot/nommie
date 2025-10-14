//! Authentication and authorization tests
//!
//! Tests for authentication flows, JWT validation, and auth extractors.
//!
//! Run all auth tests:
//!   cargo test --test auth_tests
//!
//! Run specific auth tests:
//!   cargo test --test auth_tests auth::login::

mod common;
mod support;

#[path = "suites/auth/mod.rs"]
mod auth;
