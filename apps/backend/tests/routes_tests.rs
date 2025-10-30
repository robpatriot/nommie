// HTTP routes and middleware tests
//
// Tests for HTTP handlers, extractors, validation, error formatting,
// healthcheck, and state management.
//
// Run all routes tests:
//   cargo test --test routes_tests
//
// Run specific routes tests:
//   cargo test --test routes_tests routes::handler_players::

mod common;
mod support;

#[path = "suites/routes/mod.rs"]
mod routes;
